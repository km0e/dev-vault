$packageIdsArray = $pkgs -split ' '
foreach ($packageId in $packageIdsArray) {
    $result = winget install --id $packageId
}