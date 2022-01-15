cargo build --release

if ((HOSTNAME.EXE) -eq "DESKTOP-KN4CLCI") {
	Copy-Item .\target\release\viewer.exe ~\commands\
} else {
	Write-Host "Not sure where to install"
}
