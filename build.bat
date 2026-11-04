@echo off
setlocal
cd /d "%~dp0"
where cargo >nul 2>&1 || goto norust
if exist vendor\pdfium.dll goto havedll
echo Scarico PDFium...
if not exist vendor mkdir vendor
powershell -NoProfile -Command "Invoke-WebRequest https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz -OutFile vendor\pdfium.tgz"
tar -xzf vendor\pdfium.tgz -C vendor bin/pdfium.dll
move /y vendor\bin\pdfium.dll vendor\pdfium.dll >nul
rmdir vendor\bin
del vendor\pdfium.tgz
:havedll
if not exist vendor\pdfium.dll goto nodll
call :findiscc
if defined ISCC goto build
echo Installo Inno Setup...
winget install -e --id JRSoftware.InnoSetup --accept-source-agreements --accept-package-agreements
call :findiscc
if not defined ISCC goto noinno
:build
cargo build --release || goto fail
copy /y vendor\pdfium.dll target\release\ >nul
"%ISCC%" installer\installer.iss || goto fail
echo.
echo FATTO: dist\PDFReaderCP_Setup_1.0.exe
pause
exit /b 0
:findiscc
set "ISCC="
if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if exist "%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe" set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
exit /b 0
:norust
echo Rust non trovato: installalo da https://rustup.rs e rilancia questo file.
pause
exit /b 1
:nodll
echo Impossibile scaricare pdfium.dll: scaricala da https://github.com/bblanchon/pdfium-binaries/releases e mettila in vendor\
pause
exit /b 1
:noinno
echo Inno Setup non trovato: installalo da https://jrsoftware.org/isdl.php e rilancia questo file.
pause
exit /b 1
:fail
echo ERRORE durante la compilazione.
pause
exit /b 1
