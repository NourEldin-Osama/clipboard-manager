; The Windows installer: installs Clipboard Manager for the signed-in user,
; without administrator rights, with Start menu and desktop shortcuts and an
; entry in Installed apps. The release job builds it with Inno Setup 6.5 or
; newer:
;
;   iscc /DAppVersion=0.4.0 /DAppBuild=12 /DSourceExe=..\clipboard-manager-windows-x64.exe installer\clipboard-manager.iss
;
; The app updates itself in the install folder, as the portable exe does, so
; installing a newer release is never needed for updates.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef AppBuild
  #define AppBuild "0"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\dist\clipboard-manager.exe"
#endif

[Setup]
; Keep this id: it ties every version to one entry in Installed apps. The app
; reads the same entry (installed.rs).
AppId={{2869B14A-B2DA-4F94-87D5-ED74302ED192}
AppName=Clipboard Manager
AppVersion={#AppVersion}
AppVerName=Clipboard Manager {#AppVersion}
AppPublisher=NourEldin Osama
AppPublisherURL=https://github.com/NourEldin-Osama/clipboard-manager
AppSupportURL=https://github.com/NourEldin-Osama/clipboard-manager/issues
VersionInfoVersion={#AppVersion}.{#AppBuild}
; With the lowest privileges this is %LOCALAPPDATA%\Programs.
DefaultDirName={autopf}\Clipboard Manager
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..
OutputBaseFilename=clipboard-manager-windows-x64-setup
UninstallDisplayIcon={app}\clipboard-manager.exe
UninstallDisplayName=Clipboard Manager
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; A running copy is closed, so its exe can be replaced.
CloseApplications=force
RestartApplications=no

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl,messages-en.isl"
Name: "ar"; MessagesFile: "compiler:Languages\Arabic.isl,messages-ar.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "clipboard-manager.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Clipboard Manager"; Filename: "{app}\clipboard-manager.exe"
Name: "{autodesktop}\Clipboard Manager"; Filename: "{app}\clipboard-manager.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\clipboard-manager.exe"; Description: "{cm:LaunchProgram,Clipboard Manager}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; What the app's own updater leaves next to the exe (update.rs).
Type: files; Name: "{app}\clipboard-manager.new"
Type: files; Name: "{app}\clipboard-manager.new.json"
Type: files; Name: "{app}\clipboard-manager.part"
Type: files; Name: "{app}\clipboard-manager.old"

[Code]
const
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  RunValue = 'Clipboard Manager';

{ The app may be running, in the tray say: that copy is stopped, so its exe
  can be removed. Only the copy in the install folder: a portable copy or a
  build elsewhere keeps running. }
function InitializeUninstall(): Boolean;
var
  Exe: String;
  ResultCode: Integer;
begin
  Exe := ExpandConstant('{app}\clipboard-manager.exe');
  StringChangeEx(Exe, '''', '''''', True);
  Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -NonInteractive -Command "Get-Process -Name clipboard-manager -ErrorAction SilentlyContinue' +
    ' | Where-Object { $_.Path -eq ''' + Exe + ''' } | Stop-Process -Force"',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Command, DataDir: String;
begin
  if CurUninstallStep <> usPostUninstall then
    Exit;
  { Start at sign-in, if it starts this copy. }
  if RegQueryStringValue(HKCU, RunKey, RunValue, Command) and
    (Pos(Lowercase(ExpandConstant('{app}\clipboard-manager.exe')), Lowercase(Command)) > 0) then
    RegDeleteValue(HKCU, RunKey, RunValue);
  { The history and settings, only when the user says so. A silent uninstall
    keeps them. }
  DataDir := ExpandConstant('{userappdata}\clipboard-manager');
  if DirExists(DataDir) and not UninstallSilent() and
    (MsgBox(FmtMessage(CustomMessage('DeleteData'), [DataDir]), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES) then
    DelTree(DataDir, True, True, True);
end;
