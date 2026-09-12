# Regenerate the checked-in Windows icon only when assets/logo.png changes.
# Build-time image conversion; the installed app does not use PowerShell or .NET.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$source = [Drawing.Image]::FromFile((Join-Path $root 'assets/logo.png'))
try {
    if ($source.Width -ne $source.Height) { throw 'Application logo must be square; source is preserved.' }
    $sizes = @(16,20,24,28,32,40,48,56,64,96,128,256)
    $images = @()
    foreach ($size in $sizes) {
        $bitmap = New-Object Drawing.Bitmap($size,$size,[Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
                $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                $graphics.Clear([Drawing.Color]::Transparent)
                $graphics.DrawImage($source,(New-Object Drawing.Rectangle(0,0,$size,$size)))
            } finally { $graphics.Dispose() }
            $stream = New-Object IO.MemoryStream
            try {
                $bitmap.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
                $images += ,$stream.ToArray()
            } finally { $stream.Dispose() }
        } finally { $bitmap.Dispose() }
    }
    $output = [IO.File]::Create((Join-Path $root 'assets/imirror.ico'))
    $writer = New-Object IO.BinaryWriter($output)
    try {
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
        $offset = 6 + 16 * $sizes.Count
        for ($i=0; $i -lt $sizes.Count; $i++) {
            $dimension = [byte]($sizes[$i] % 256)
            $writer.Write($dimension); $writer.Write($dimension)
            $writer.Write([byte]0); $writer.Write([byte]0)
            $writer.Write([uint16]1); $writer.Write([uint16]32)
            $writer.Write([uint32]$images[$i].Length); $writer.Write([uint32]$offset)
            $offset += $images[$i].Length
        }
        foreach ($bytes in $images) { $writer.Write([byte[]]$bytes) }
    } finally { $writer.Dispose() }
    Write-Output ('Generated assets/imirror.ico: '+($sizes -join ', ')+' px; original logo.png unchanged.')
} finally { $source.Dispose() }
