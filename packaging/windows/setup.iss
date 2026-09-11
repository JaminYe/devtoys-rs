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

[InstallDelete]
Type: files; Name: "{app}\devtoys-cli.exe"
Type: files; Name: "{group}\{#MyAppName} CLI.lnk"

[Files]
Source: "..\..\target\{#Target}\release\devtoys.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\crates\host\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\devtoys.exe"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\devtoys.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\devtoys.exe"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[Code]
function OpenProcess(dwDesiredAccess: DWORD; bInheritHandle: BOOL; dwProcessId: DWORD): THandle;
  external 'OpenProcess@kernel32.dll stdcall';
function CloseHandle(hObject: THandle): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';
function WaitForSingleObject(hHandle: THandle; dwMilliseconds: DWORD): DWORD;
  external 'WaitForSingleObject@kernel32.dll stdcall';

const
  SYNCHRONIZE = $00100000;
  WAIT_TIMEOUT = $00000102;

var
  IsUpdateMode: Boolean;
  HandshakeFile: String;
  CallerPID: LongInt;
  BackupExePath: String;
  TargetExePath: String;

function InitializeSetup(): Boolean;
var
  HProc: THandle;
  WaitResult: DWORD;
  Attempts: Integer;
  FileContent: AnsiString;
begin
  Result := True;
  IsUpdateMode := (ExpandConstant('{param:UPDATE_MODE|0}') = '1');
  
  if IsUpdateMode then
  begin
    HandshakeFile := ExpandConstant('{param:HANDSHAKE_FILE|}');
    CallerPID := StrToIntDef(ExpandConstant('{param:CALLER_PID|0}'), 0);

    // 1. Notify caller application via handshake file that installer is elevated & ready
    if (HandshakeFile <> '') then
    begin
      SaveStringToFile(HandshakeFile, 'READY' + #13#10, False);
    end;

    // 2. Wait up to 15 seconds for caller process to exit gracefully, or for abort signal
    if CallerPID > 0 then
    begin
      HProc := OpenProcess(SYNCHRONIZE, False, CallerPID);
      if HProc <> 0 then
      begin
        Attempts := 0;
        while Attempts < 150 do
        begin
          WaitResult := WaitForSingleObject(HProc, 100);
          if WaitResult = 0 then
          begin
            // Caller process exited cleanly
            Break;
          end;

          // Check for immediate ABORT signal from host application
          if (HandshakeFile <> '') and FileExists(HandshakeFile) then
          begin
            if LoadStringFromFile(HandshakeFile, FileContent) then
            begin
              if Pos('ABORT', FileContent) > 0 then
              begin
                Log('Update aborted: host application signaled ABORT.');
                CloseHandle(HProc);
                Result := False;
                Exit;
              end;
            end;
          end;

          Attempts := Attempts + 1;
        end;

        CloseHandle(HProc);

        if Attempts >= 150 then
        begin
          Log('Update aborted: caller process failed to exit within 15 seconds.');
          Result := False;
          Exit;
        end;
      end;
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if IsUpdateMode then
  begin
    TargetExePath := ExpandConstant('{app}\devtoys.exe');
    BackupExePath := ExpandConstant('{app}\devtoys.exe.bak');

    if CurStep = ssInstall then
    begin
      // Backup existing binary before replacement
      if FileExists(TargetExePath) then
      begin
        FileCopy(TargetExePath, BackupExePath, False);
      end;
    end
    else if CurStep = ssPostInstall then
    begin
      // Installation succeeded, remove backup
      if FileExists(BackupExePath) then
      begin
        DeleteFile(BackupExePath);
      end;

      // Launch newly installed executable as the original (non-elevated) user
      if FileExists(TargetExePath) then
      begin
        ExecAsOriginalUser(TargetExePath, '', '', SW_SHOWNORMAL, ewNoWait, ResultCode);
      end;
    end;
  end;
end;

procedure DeinitializeSetup();
var
  ResultCode: Integer;
begin
  if IsUpdateMode then
  begin
    // If backup still exists, installation failed or was aborted before post-install
    if FileExists(BackupExePath) then
    begin
      Log('Installation failed or aborted; restoring backup executable.');
      FileCopy(BackupExePath, TargetExePath, False);
      DeleteFile(BackupExePath);
      // Relaunch the restored original executable as original user
      if FileExists(TargetExePath) then
      begin
        ExecAsOriginalUser(TargetExePath, '', '', SW_SHOWNORMAL, ewNoWait, ResultCode);
      end;
    end;
  end;
end;
