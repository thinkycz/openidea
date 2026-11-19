use super::*;
use futures::FutureExt;

const ARTICLE: &str = r#"<?php namespace App\Models;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;
class Article extends Model {
    use HasFactory;
    protected $guarded = [];
    /* 😀 žluťoučký */ public function articleProbe(): string { return 'article'; }
}
"#;
const EDITION: &str = r#"<?php namespace App\Models;
use Illuminate\Database\Eloquent\Model;
class Edition extends Model {
    protected $guarded = [];
    /* 😀 */ public function editionProbe(): string { return 'edition'; }
}
"#;
const FACTORY: &str = r#"<?php namespace Database\Factories;
use Illuminate\Database\Eloquent\Factories\Factory;
class ArticleFactory extends Factory {
    /* 😀 state */ public function featured(): static { return $this->state(['title' => 'featured']); }
    public function definition(): array { return ['title' => 'sample']; }
}
"#;
const EDITORIAL: &str = r#"<?php namespace Database\Factories;
use Illuminate\Database\Eloquent\Factories\Factory;
class EditorialFactory extends Factory {
    protected $model = \App\Models\Article::class;
    public function editorialState(): static { return $this; }
    public function definition(): array { return ['title' => 'editorial']; }
}
"#;
const CALLER: &str = r#"<?php namespace App;
use App\Models\Article;
function inspect(): void {
    Article::factory()->featured()->make()->articleProbe();
    Article::factory(2)->make()->first()->articleProbe();
}
"#;

async fn factory_members(mgr: &LspManager, doc: DocId, text: &str, needle: &str, required: &str, forbidden: &str) {
    let at = pos(text, needle);
    let at = Pos::new(at.line, at.col + needle.encode_utf16().count() as u32);
    let list = settled_completion(mgr, doc, at).await;
    let names: Vec<_> = list.items.iter().map(|item| item.label.split('(').next().unwrap()).collect();
    assert!(names.contains(&required) && !names.contains(&forbidden), "factory members: {needle}: {names:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and OPENSTORM_TEST_FACTORY_VENDOR with Laravel 13.37 dependencies"]
async fn factories_keep_states_models_unsaved_selection_origins_and_restart_through_client() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"));
    assert!(vendor.join("laravel/framework/src/Illuminate/Database/Eloquent/Factories/Factory.php").is_file());
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    let framework = installed["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "laravel/framework")
        .unwrap();
    assert!(matches!(framework["version"].as_str(), Some("v13.37.0" | "13.37.0")));
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for directory in ["app/Models", "database/factories"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    std::os::unix::fs::symlink(vendor.canonicalize().unwrap(), root.join("vendor")).unwrap();
    let files = [
        (
            "composer.json",
            r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/","Database\\Factories\\":"database/factories/"}}}"#,
        ),
        ("app/Models/Article.php", ARTICLE),
        ("app/Models/Edition.php", EDITION),
        ("database/factories/ArticleFactory.php", FACTORY),
        ("database/factories/EditorialFactory.php", EDITORIAL),
        ("app/FactoryProbe.php", CALLER),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let caller = DocId::next();
    let model = DocId::next();
    let factory = DocId::next();
    mgr.did_open(caller, &root.join("app/FactoryProbe.php"), &LanguageId::new("php"), 1, CALLER);
    mgr.did_open(model, &root.join("app/Models/Article.php"), &LanguageId::new("php"), 1, ARTICLE);
    mgr.did_open(factory, &root.join("database/factories/ArticleFactory.php"), &LanguageId::new("php"), 1, FACTORY);
    let outcome = std::panic::AssertUnwindSafe(async {
        factory_members(&mgr, caller, CALLER, "factory()->", "featured", "editorialState").await;
        factory_members(&mgr, caller, CALLER, "make()->", "articleProbe", "editionProbe").await;
        factory_members(&mgr, caller, CALLER, "make()->", "articleProbe", "map").await;
        factory_members(&mgr, caller, CALLER, "factory(2)->make()->", "map", "articleProbe").await;
        factory_members(&mgr, caller, CALLER, "first()->", "articleProbe", "editionProbe").await;
        for (needle, path, source, declaration) in [
            ("featured()->", "database/factories/ArticleFactory.php", FACTORY, "featured()"),
            ("articleProbe();", "app/Models/Article.php", ARTICLE, "articleProbe()"),
        ] {
            let targets = mgr.goto(caller, pos(CALLER, needle), GotoKind::Definition).await.unwrap();
            assert!(targets.iter().any(|target| *target.path == root.join(path) && target.range.start == pos(source, declaration)), "factory UTF16 origin: {targets:?}");
        }
        let dirty_factory = FACTORY.replace("public function featured", "protected $model = \\App\\Models\\Edition::class;\n    public function seasonal");
        let dirty_caller = CALLER.replace("featured()", "seasonal()").replace("articleProbe()", "editionProbe()");
        replace(&mgr, factory, 2, FACTORY, &dirty_factory);
        replace(&mgr, caller, 2, CALLER, &dirty_caller);
        factory_members(&mgr, caller, &dirty_caller, "factory()->", "seasonal", "featured").await;
        factory_members(&mgr, caller, &dirty_caller, "make()->", "editionProbe", "articleProbe").await;
        factory_members(&mgr, caller, &dirty_caller, "factory(2)->make()->", "map", "editionProbe").await;
        factory_members(&mgr, caller, &dirty_caller, "first()->", "editionProbe", "articleProbe").await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        factory_members(&mgr, caller, &dirty_caller, "make()->", "editionProbe", "articleProbe").await;
        mgr.did_close(factory);
        replace(&mgr, caller, 3, &dirty_caller, CALLER);
        factory_members(&mgr, caller, CALLER, "factory()->", "featured", "seasonal").await;
        factory_members(&mgr, caller, CALLER, "make()->", "articleProbe", "editionProbe").await;

        let attributed = ARTICLE.replace("class Article extends Model", "#[\\Illuminate\\Database\\Eloquent\\Attributes\\UseFactory(\\Database\\Factories\\EditorialFactory::class)]\nclass Article extends Model");
        let editorial = CALLER.replace("featured()", "editorialState()");
        replace(&mgr, model, 2, ARTICLE, &attributed);
        replace(&mgr, caller, 4, CALLER, &editorial);
        factory_members(&mgr, caller, &editorial, "factory()->", "editorialState", "featured").await;
        let targets = mgr.goto(caller, pos(&editorial, "editorialState()"), GotoKind::Definition).await.unwrap();
        assert!(targets.iter().any(|target| *target.path == root.join("database/factories/EditorialFactory.php") && target.range.start == pos(EDITORIAL, "editorialState()")), "attribute factory origin: {targets:?}");
        let overridden = attributed.replace("class Article extends Model {", "class Article extends Model {\n protected static function newFactory(): \\Database\\Factories\\ArticleFactory { return \\Database\\Factories\\ArticleFactory::new(); }");
        replace(&mgr, model, 3, &attributed, &overridden);
        replace(&mgr, caller, 5, &editorial, CALLER);
        factory_members(&mgr, caller, CALLER, "factory()->", "featured", "editorialState").await;
        mgr.did_close(model);
        factory_members(&mgr, caller, CALLER, "factory()->", "featured", "editorialState").await;
        let uncounted = CALLER.replace("factory(2)->make()->first()", "factory(2)->count(null)->make()");
        replace(&mgr, caller, 6, CALLER, &uncounted);
        factory_members(&mgr, caller, &uncounted, "count(null)->make()->", "articleProbe", "editionProbe").await;
        factory_members(&mgr, caller, &uncounted, "count(null)->make()->", "articleProbe", "map").await;
        mgr.did_close(caller);
    }).catch_unwind().await;
    mgr.shutdown_all().await;
    for (path, original) in files {
        assert_eq!(std::fs::read_to_string(root.join(path)).unwrap(), original);
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
