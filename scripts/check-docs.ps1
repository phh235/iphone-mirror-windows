Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent)).TrimEnd('\','/')
$files = @(Get-ChildItem -LiteralPath $root -Filter '*.md' -File)
foreach ($folder in @('docs','assets','.github')) {
    $path = Join-Path $root $folder
    if (Test-Path -LiteralPath $path) { $files += Get-ChildItem -LiteralPath $path -Filter '*.md' -File -Recurse }
}
$missing = [Collections.Generic.List[string]]::new()
$checked = 0
$utf8 = New-Object Text.UTF8Encoding($false,$true)
foreach ($file in $files) {
    $content = $utf8.GetString([IO.File]::ReadAllBytes($file.FullName))
    $links = [regex]::Matches($content,'\]\((?<path><[^>]+>|[^\s)]+)(?:\s+"[^"]*")?\)|(?:src|href)="(?<path>[^"]+)"')
    foreach ($match in $links) {
        $path = $match.Groups['path'].Value.Trim('<','>')
        if ($path -match '^(#|[a-zA-Z][a-zA-Z0-9+.-]*:|//)') { continue }
        $path = [Uri]::UnescapeDataString(($path -split '[#?]',2)[0])
        if (-not $path) { continue }
        $target = [IO.Path]::GetFullPath((Join-Path $file.DirectoryName $path))
        $checked++
        if (-not $target.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or
            -not (Test-Path -LiteralPath $target)) {
            $relative = $file.FullName.Substring($root.Length+1)
            $line = 1 + ([regex]::Matches($content.Substring(0,$match.Index),"`n")).Count
            $missing.Add("${relative}:${line}: $path")
        }
    }
}
if ($missing.Count) { throw ("Broken local documentation links:`n"+($missing -join "`n")) }
Write-Output ("PASS: $($files.Count) UTF-8 Markdown files, $checked local file links. External URLs and heading anchors are not checked.")
