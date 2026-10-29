/** Whether the Unicode code point before a UTF-16 offset continues an identifier. */
export function hasIdentifierBefore(text: string, offset: number): boolean {
  // Read one complete code point without lookbehind (older WebKit) or an
  // end-anchored Unicode class (which mishandles astral letters in Node 26).
  const previous = [...text.slice(Math.max(0, offset - 2), offset)].at(-1);
  return previous !== undefined && /[\p{L}\p{N}_$]/u.test(previous);
}
