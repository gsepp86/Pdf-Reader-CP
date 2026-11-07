; Inno Setup 6 - https://jrsoftware.org/isinfo.php
#define AppName "PDF Reader CP"
#define AppVer "2.0"
#define Exe "PDFReaderCP.exe"

[Setup]
AppId={{B7C1E6A2-5D4F-4C0B-9A57-6E2D1F3A9C10}
AppName={#AppName}
AppVersion={#AppVer}
AppPublisher=Peppe CP
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
OutputDir=..\dist
OutputBaseFilename=PDFReaderCP_Setup_{#AppVer}
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#Exe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "Crea un'icona sul Desktop"; GroupDescription: "Icone aggiuntive:"
Name: "pdfassoc"; Description: "Apri i file .pdf con {#AppName}"; GroupDescription: "Associazioni:"; Flags: unchecked

[Files]
Source: "..\target\release\{#Exe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\vendor\pdfium.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#Exe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#Exe}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.pdf\OpenWithProgids"; ValueType: string; ValueName: "PDFReaderCP.pdf"; ValueData: ""; Flags: uninsdeletevalue; Tasks: pdfassoc
Root: HKA; Subkey: "Software\Classes\PDFReaderCP.pdf"; ValueType: string; ValueData: "Documento PDF"; Flags: uninsdeletekey; Tasks: pdfassoc
Root: HKA; Subkey: "Software\Classes\PDFReaderCP.pdf\shell\open\command"; ValueType: string; ValueData: """{app}\{#Exe}"" ""%1"""; Tasks: pdfassoc

[Run]
Filename: "{app}\{#Exe}"; Description: "Avvia {#AppName}"; Flags: nowait postinstall skipifsilent
