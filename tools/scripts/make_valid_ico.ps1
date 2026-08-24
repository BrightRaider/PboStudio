Add-Type -AssemblyName System.Drawing

$pngPath = "C:\Users\Ionas\APP\PboStudio\src\PboStudio.App\icon.png"
$icoPath = "C:\Users\Ionas\APP\PboStudio\src\PboStudio.App\icon.ico"

$bmp = [System.Drawing.Bitmap]::FromFile($pngPath)
$hIcon = $bmp.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($hIcon)

$stream = [System.IO.File]::Create($icoPath)
$icon.Save($stream)
$stream.Close()
$icon.Dispose()
$bmp.Dispose()

Write-Host "Valid native ICO generated at $icoPath"
