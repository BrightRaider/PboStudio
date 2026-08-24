[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$wc = New-Object System.Net.WebClient
$wc.Headers.Add("User-Agent", "Mozilla/5.0")

$urls = @(
    "https://github.com/namazso/PawnIO.Setup/releases/download/v1.0.0/PawnIO_setup.exe",
    "https://github.com/namazso/PawnIO.Setup/releases/download/v1.0.0.0/PawnIO_setup.exe",
    "https://github.com/namazso/PawnIO.Setup/releases/download/1.0.0/PawnIO_setup.exe",
    "https://pawnio.eu/PawnIO_setup.exe"
)

foreach ($url in $urls) {
    try {
        Write-Host "Trying $url ..."
        $wc.DownloadFile($url, (Join-Path $PSScriptRoot "..\..\PawnIO_setup.exe"))
        Write-Host "SUCCESS! Downloaded to $PSScriptRoot\..\..\PawnIO_setup.exe"
        exit 0
    } catch {
        Write-Host "Failed: $_"
    }
}

# If static links fail, query GitHub API
try {
    $api = "https://api.github.com/repos/namazso/PawnIO.Setup/releases/latest"
    $json = $wc.DownloadString($api) | ConvertFrom-Json
    $asset = $json.assets | Where-Object { $_.name -like "*.exe" } | Select-Object -First 1
    if ($asset) {
        Write-Host "Found API asset: $($asset.browser_download_url)"
        $wc.DownloadFile($asset.browser_download_url, (Join-Path $PSScriptRoot "..\..\PawnIO_setup.exe"))
        Write-Host "SUCCESS from API!"
        exit 0
    }
} catch {
    Write-Host "API Failed: $_"
}
