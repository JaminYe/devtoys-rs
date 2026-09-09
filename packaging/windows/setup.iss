; Inno Setup script for DevToys-RS (x64)
; 编译参数：ISCC /DAppVersion=x.y.z /DTarget=x86_64-pc-windows-msvc setup.iss

#define MyAppName "DevToys"

#ifndef AppVersion
#define AppVersion "0.0.0"
#endif

#ifndef Target
#define Target "x86_64-pc-windows-msvc"
#endif

[Setup]
AppId={{9E3C1D42-58A7-4B61-B0AE-F3621C79A5D8}}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher=JaminYe
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=output
OutputBaseFilename=devtoys-{#Target}-setup
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=no
CloseApplications=no
SetupIconFile=..\..\crates\host\assets\icon.ico
UninstallDisplayIcon={app}\devtoys.exe

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
; 中文语言包为非官方翻译，runner 上可能不存在，存在时才启用
#if FileExists(CompilerPath + "Languages\ChineseSimplified.isl")
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
#endif

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\..\target\{#Target}\release\devtoys.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\{#Target}\release\devtoys-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\crates\host\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\devtoys.exe"
Name: "{group}\{#MyAppName} CLI"; Filename: "cmd.exe"; Parameters: "/K ""{app}\devtoys-cli.exe"" --help"; IconFilename: "{app}\devtoys.exe"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\devtoys.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\devtoys.exe"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
