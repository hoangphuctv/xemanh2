#define AppName "XemAnh2"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#define AppPublisher "XemAnh2"
#define AppExeName "xemanh2.exe"
#define OutputName "xemanh2-" + AppVersion + "-setup"

[Setup]
AppId={{8E1C8E7B-6A0A-4C61-9A2D-9F4A2A8D8F51}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\XemAnh2
DefaultGroupName={#AppName}
OutputDir=dist
OutputBaseFilename={#OutputName}
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
UninstallDisplayIcon={app}\{#AppExeName}

[Files]
Source: "target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
