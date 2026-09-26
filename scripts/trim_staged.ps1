# Trim trailing whitespace in staged files (mirrors the prek hook) so commits
# don't abort. Usage: pwsh -File scripts/trim_staged.ps1
foreach ($f in (git diff --cached --name-only)) {
    if (-not (Test-Path $f)) { continue }
    $c = [IO.File]::ReadAllText($f)
    $n = ($c -split "`n" | ForEach-Object { $_.TrimEnd() }) -join "`n"
    if ($n -ne $c) {
        [IO.File]::WriteAllText($f, $n)
        git add $f
        Write-Output "trimmed: $f"
    }
}
