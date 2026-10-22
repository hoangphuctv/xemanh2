#define AppName "XemAnh2"
#define AppVersion GetEnv("XEMANH2_VERSION")
#if AppVersion == ""
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
SetupIconFile=assets\icon.ico
DefaultDirName={autopf}\XemAnh2
DefaultGroupName={#AppName}
OutputDir=dist
OutputBaseFilename={#OutputName}
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
ChangesAssociations=yes
UninstallDisplayIcon={app}\{#AppExeName}

[Files]
Source: "target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
; XemAnh2 image file association.
Root: HKCU; Subkey: "Software\Classes\XemAnh2.Image"; ValueType: string; ValueName: ""; ValueData: "XemAnh2 Image"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\XemAnh2.Image\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\icon.ico,0"
Root: HKCU; Subkey: "Software\Classes\XemAnh2.Image\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""

Root: HKCU; Subkey: "Software\Classes\.jpg"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.jpeg"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.png"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.bmp"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.gif"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.webp"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"
Root: HKCU; Subkey: "Software\Classes\.tga"; ValueType: string; ValueName: ""; ValueData: "XemAnh2.Image"

Root: HKCU; Subkey: "Software\Classes\.jpg\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.png\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.bmp\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.gif\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.webp\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.tga\OpenWithProgids"; ValueType: string; ValueName: "XemAnh2.Image"; ValueData: ""; Flags: uninsdeletevalue

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
procedure SHChangeNotify(wEventId: Integer; uFlags: Integer; dwItem1, dwItem2: Integer);
  external 'SHChangeNotify@shell32.dll stdcall';

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    SHChangeNotify($08000000, 0, 0, 0);
end;
