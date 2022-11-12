$modbreeze_dir = "$Env:USERPROFILE\.modbreeze"

# create ~/.modbreeze/bin and parents
if (-not (Test-Path "$modbreeze_dir\bin")) {
    mkdir -p "$modbreeze_dir\bin" > $null
    echo "Created ~/.modbreeze."
}

# download zip archive from latest github release
echo "Downloading..."
curl https://github.com/Mr1cecream/ModBreeze/releases/download/v0.3.0/modbreeze-windows.zip `
    -o "$modbreeze_dir\bin\modbreeze.zip"

# extract binary file from zip archive and remove archive
echo "Extracting..."
Expand-Archive "$modbreeze_dir\bin\modbreeze.zip" -DestinationPath "$modbreeze_dir\bin\"
Remove-Item "$modbreeze_dir\bin\modbreeze.zip"

# add ~/.modbreeze/bin to PATH if it's not present already
if ( -not ($Env:PATH -split ";" -contains "$modbreeze_dir\bin")) {
    echo "`$Env:PATH = `"`$Env:PATH;$modbreeze_dir\bin`"" >> $profile
    echo "Added ~/.modbreeze/bin to PATH."
}

# set config path to ~/.modbreeze if it is not set
if (-not (Test-Path env:MODBREEZE_CONFIG_PATH)) {
    echo "`$Env:MODBREEZE_CONFIG_PATH = `"$modbreeze_dir`"" >> $profile
}

# refresh $profile to add the path
. $profile

echo "Done installing Modbreeze. Run ``modbreeze -h`` for help."
