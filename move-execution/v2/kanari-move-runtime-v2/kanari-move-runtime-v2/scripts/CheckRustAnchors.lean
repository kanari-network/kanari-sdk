import KanariMoveRuntime.Runtime.RustApplyCorrespondence

open KanariMoveRuntime.Runtime.RustApply

/-- Check one anchor against the current Rust source. The check verifies that
the file exists, the range is inside the file, and the stable marker token
still occurs inside the declared range. It detects shifted or deleted anchors;
it is not a proof that the initial transcription was faithful. -/
def checkAnchor (a : RustAnchor) : IO (List String) := do
  let path : System.FilePath := "../" ++ a.file
  let lines ← IO.FS.lines path
  let n := lines.size
  if a.first < 1 ∨ a.last < a.first then
    return [s!"invalid range in {a.file}: {a.first}-{a.last}"]
  else if a.last > n then
    return [s!"short file {a.file}: {n} lines, anchor ends at {a.last}"]
  else
    let slice := (lines.toList.drop (a.first - 1)).take (a.last - a.first + 1)
    if slice.any (fun line => line.contains a.marker) then
      return []
    else
      return [s!"marker missing in {a.file}:{a.first}-{a.last}: {a.marker}"]

def main : IO Unit := do
  let mut errors : List String := []
  for a in rustAnchors do
    errors := errors ++ (← checkAnchor a)
  if errors.length == 0 then
    IO.println s!"Rust anchor check passed: {rustAnchors.length} anchors"
  else
    for e in errors do
      IO.eprintln e
    IO.Process.exit 1
