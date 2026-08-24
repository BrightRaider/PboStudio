Add-Type -AssemblyName System.Drawing

$jpgPath = (Join-Path $PSScriptRoot "icon-source.jpg")
$pngPath = (Join-Path $PSScriptRoot "..\..\src\PboStudio.App\icon.png")

$img = [System.Drawing.Image]::FromFile($jpgPath)
# Resize to a clean 256x256 icon bitmap
$bmp = New-Object System.Drawing.Bitmap(256, 256)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.DrawImage($img, 0, 0, 256, 256)
$g.Dispose()
$img.Dispose()

$bmp.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Host "Valid PNG icon generated at $pngPath"
