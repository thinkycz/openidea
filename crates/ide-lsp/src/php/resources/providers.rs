use super::*;
use std::io::Read;

const CAPABILITY: &str = "/experimental/phpWorkbenchProviderSources";
const MAX_PROVIDERS: usize = 512;
const MAX_BYTES: u64 = 1_048_576;
const MAX_TOTAL_BYTES: usize = 8 * 1_048_576;

/// A statically resolved provider file and its UTF-8 source digest in the selected server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhpProviderSource {
    pub class_name: String,
    pub path: PathBuf,
    pub sha256_utf8: [u8; 32],
}

/// Known provider locations, not an exhaustive runtime dependency graph.
pub struct PhpProviderSources {
    pub providers: Vec<PhpProviderSource>,
    pub unresolved: Vec<String>,
    pub limited: bool,
    inputs: Vec<NavigationSource>,
    files: Vec<(PathBuf, Stamp)>,
}

impl std::fmt::Debug for PhpProviderSources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhpProviderSources")
            .field("providers", &self.providers)
            .field("unresolved", &self.unresolved)
            .field("limited", &self.limited)
            .finish_non_exhaustive()
    }
}

impl PhpProviderSources {
    /// Reports unresolved static names or a bound, not runtime registration coverage.
    pub fn incomplete(&self) -> bool {
        self.limited || !self.unresolved.is_empty()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireSources {
    providers: Vec<WireSource>,
    unresolved: Vec<String>,
    limited: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireSource {
    class_name: String,
    uri: String,
    sha256: String,
}

#[cfg(unix)]
type Stamp = ReviewedDiskMetadata;

#[cfg(not(unix))]
#[derive(PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    readonly: bool,
}

#[cfg(not(unix))]
impl Stamp {
    fn new(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            readonly: metadata.permissions().readonly(),
        }
    }
}

pub(super) fn supported(caps: &ServerCaps) -> bool {
    caps.raw.pointer(CAPABILITY).and_then(Value::as_bool) == Some(true)
}

fn provider_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 4096
        && name.split('\\').all(|part| {
            !part.is_empty()
                && !part.as_bytes()[0].is_ascii_digit()
                && part.bytes().all(|byte| byte >= 0x80 || byte.is_ascii_alphanumeric() || byte == b'_')
        })
}

fn parse_hash(value: &str) -> IdeResult<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(IdeError::invalid("Invalid provider source digest"));
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| IdeError::invalid("Invalid provider source digest"))?;
    }
    Ok(digest)
}

fn parse(value: Value) -> IdeResult<PhpProviderSources> {
    let count = |name| value.get(name).and_then(Value::as_array).map(Vec::len);
    if count("providers").zip(count("unresolved")).is_none_or(|(a, b)| a.saturating_add(b) > MAX_PROVIDERS) {
        return Err(IdeError::invalid("Invalid bounded provider source inventory"));
    }
    let wire: WireSources =
        serde_json::from_value(value).map_err(|_| IdeError::invalid("Invalid provider source metadata"))?;
    let mut names = HashSet::new();
    let mut hashes = std::collections::HashMap::new();
    let mut providers = Vec::new();
    for source in wire.providers {
        if !provider_name(&source.class_name)
            || !names.insert(source.class_name.to_ascii_lowercase())
            || source.uri.len() > 16_384
        {
            return Err(IdeError::invalid("Invalid provider source identity"));
        }
        let path = uri::uri_to_path(&source.uri)
            .filter(|path| uri::path_to_uri_string(path) == source.uri)
            .ok_or_else(|| IdeError::invalid("Expected a physical provider source URI"))?;
        let sha256_utf8 = parse_hash(&source.sha256)?;
        if hashes.insert(path.clone(), sha256_utf8).is_some_and(|previous| previous != sha256_utf8) {
            return Err(IdeError::invalid("Conflicting provider source digests"));
        }
        providers.push(PhpProviderSource { class_name: source.class_name, path, sha256_utf8 });
    }
    for name in &wire.unresolved {
        if !provider_name(name) || !names.insert(name.to_ascii_lowercase()) {
            return Err(IdeError::invalid("Invalid unresolved provider identity"));
        }
    }
    Ok(PhpProviderSources {
        providers,
        unresolved: wire.unresolved,
        limited: wire.limited,
        inputs: Vec::new(),
        files: Vec::new(),
    })
}

fn read_source(
    path: &Path,
    managed: Option<&DocText>,
    cancel: &CancellationToken,
    remaining: usize,
) -> IdeResult<(Stamp, [u8; 32], usize)> {
    physical_path(path, true)?;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| IdeError::invalid("Provider source is unavailable"))?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err(IdeError::invalid("Expected a bounded regular provider source"));
    }
    let charged = managed.map_or(metadata.len() as usize, |text| text.as_str().len().max(metadata.len() as usize));
    if charged > remaining {
        return Err(IdeError::invalid("Provider sources exceed the hashing budget"));
    }
    let stamp = Stamp::new(&metadata);
    let (digest, bytes) = if let Some(text) = managed {
        if text.as_str().len() as u64 > MAX_BYTES {
            return Err(IdeError::invalid("Provider buffer exceeds the source limit"));
        }
        (Sha256::digest(text.as_str().as_bytes()).into(), text.as_str().len().max(metadata.len() as usize))
    } else {
        #[cfg(unix)]
        let mut file = {
            use rustix::fs::{Mode, OFlags, open};
            let fd = open(path, OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK, Mode::empty())
                .map_err(|_| IdeError::invalid("Provider source cannot be opened safely"))?;
            std::fs::File::from(fd)
        };
        #[cfg(not(unix))]
        let mut file = std::fs::File::open(path).map_err(|_| IdeError::invalid("Provider source cannot be opened"))?;
        let opened = file.metadata().map_err(|_| IdeError::invalid("Provider metadata is unavailable"))?;
        if !opened.is_file() || Stamp::new(&opened) != stamp {
            return Err(IdeError::cancelled());
        }
        let mut content = Vec::with_capacity(metadata.len() as usize);
        (&mut file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut content)
            .map_err(|_| IdeError::invalid("Provider source cannot be read"))?;
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        if std::str::from_utf8(&content).is_err() {
            return Err(IdeError::invalid("Provider source is not UTF-8"));
        }
        let after = file.metadata().map_err(|_| IdeError::invalid("Provider metadata is unavailable"))?;
        if content.len() as u64 != metadata.len() || Stamp::new(&after) != stamp {
            return Err(IdeError::cancelled());
        }
        (Sha256::digest(&content).into(), content.len())
    };
    physical_path(path, true)?;
    let after = std::fs::symlink_metadata(path).map_err(|_| IdeError::invalid("Provider source authority changed"))?;
    if !after.is_file() || Stamp::new(&after) != stamp {
        return Err(IdeError::cancelled());
    }
    Ok((stamp, digest, bytes))
}

impl LspManager {
    fn provider_inputs(&self, sources: &PhpProviderSources) -> Vec<NavigationSource> {
        let paths: HashSet<_> = sources.providers.iter().map(|source| &source.path).collect();
        self.docs
            .lock()
            .iter()
            .filter(|(_, doc)| paths.contains(&doc.path) || paths.contains(&doc.canonical))
            .map(|(id, doc)| NavigationSource {
                doc: *id,
                version: doc.version,
                uri: doc.uri.clone(),
                language: doc.language.clone(),
                text: doc.text.clone(),
            })
            .collect()
    }

    fn provider_buffers_current(&self, sources: &PhpProviderSources) -> bool {
        let current = self.provider_inputs(sources);
        current.len() == sources.inputs.len()
            && sources.inputs.iter().all(|before| {
                current.iter().any(|after| {
                    before.doc == after.doc
                        && before.version == after.version
                        && before.uri == after.uri
                        && before.language == after.language
                        && Arc::ptr_eq(&before.text, &after.text)
                })
            })
    }

    async fn provider_files(
        &self,
        receipt: &PhpJsonTranslationResource,
        sources: &PhpProviderSources,
        cancel: &CancellationToken,
    ) -> IdeResult<Vec<(PathBuf, Stamp)>> {
        let mut files = std::collections::BTreeMap::new();
        for source in &sources.providers {
            files.insert(source.path.clone(), source.sha256_utf8);
        }
        let mut managed = std::collections::HashMap::new();
        for source in &receipt.sources.sources {
            if let Some(path) = uri::uri_to_path(source.uri.as_str())
                && files.contains_key(&path)
            {
                if source.language.as_str() != "php" {
                    return Err(IdeError::invalid("Provider source has a non-PHP editor owner"));
                }
                managed.insert(path, source.text.clone());
            }
        }
        let worker_cancel = cancel.clone();
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(IdeError::cancelled()),
            result = tokio::task::spawn_blocking(move || {
                let mut result = Vec::new();
                let mut bytes = 0usize;
                for (path, expected) in files {
                    if worker_cancel.is_cancelled() { return Err(IdeError::cancelled()); }
                    let (stamp, digest, read) = read_source(&path, managed.get(&path).map(Arc::as_ref), &worker_cancel, MAX_TOTAL_BYTES.saturating_sub(bytes))?;
                    bytes = bytes.saturating_add(read);
                    if bytes > MAX_TOTAL_BYTES { return Err(IdeError::invalid("Provider sources exceed the hashing budget")); }
                    if digest != expected { return Err(IdeError::new(ErrorKind::OutOfSync, "Provider source digest changed")); }
                    result.push((path, stamp));
                }
                Ok(result)
            }) => result.map_err(|error| IdeError::internal(error.to_string()))?,
        }
    }

    pub(super) async fn read_provider_sources(
        &self,
        receipt: &PhpJsonTranslationResource,
        value: Value,
        cancel: &CancellationToken,
    ) -> IdeResult<PhpProviderSources> {
        let mut sources = parse(value)?;
        sources.files = self.provider_files(receipt, &sources, cancel).await?;
        sources.inputs = self.provider_inputs(&sources);
        if sources.inputs.len() > 1024 {
            return Err(IdeError::invalid("Too many managed provider buffers"));
        }
        Ok(sources)
    }

    pub(super) async fn provider_sources_current(
        &self,
        receipt: &PhpJsonTranslationResource,
        sources: &PhpProviderSources,
        cancel: &CancellationToken,
    ) -> IdeResult<()> {
        if !supported(&receipt.target.caps()) || !self.provider_buffers_current(sources) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Provider source inputs changed"));
        }
        if self.provider_files(receipt, sources, cancel).await? != sources.files
            || !self.provider_buffers_current(sources)
        {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Provider source authority changed"));
        }
        Ok(())
    }
}
