; installer/maripdf.iss — Instalador por usuário do MariPDF (BF-603/604).
;
; Sem admin e sem UAC: instala em %LOCALAPPDATA%\Programs\MariPDF e grava só
; em HKCU. A associação .pdf usa exatamente as chaves de
; src/platform/file_assoc.rs (ProgID, OpenWithProgids, Capabilities,
; RegisteredApplications) e nunca força o padrão: o Windows exige que o
; usuário confirme em "Aplicativos padrão".
;
; Compilar (tools/package.ps1 faz isso):
;   ISCC.exe /DAppVersion=1.0.0 /DSourceExe=..\target\release\maripdf.exe installer\maripdf.iss

#ifndef AppVersion
  #error Defina /DAppVersion=<versão> (tools/package.ps1 lê do Cargo.toml)
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\maripdf.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif

#define AppName "MariPDF"
#define AppExe "maripdf.exe"
#define ProgId "MariPDF.Document"
#define AppIdGuid "73F72B7B-8E4F-4704-8CBA-9FB568284922"

[Setup]
AppId={{{#AppIdGuid}}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Flávio Takemoto
AppPublisherURL=https://github.com/flavioflavioflavio/MariPDF
AppSupportURL=https://github.com/flavioflavioflavio/MariPDF/issues
AppUpdatesURL=https://github.com/flavioflavioflavio/MariPDF/releases
AppCopyright=© 2026 Flávio Takemoto — GPL-3.0-or-later
AppComments=Dedicado a Mariana Lima, com muito amor.
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=MariPDF-{#AppVersion}-setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
LicenseFile=..\LICENSE
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
RestartApplications=no
ChangesAssociations=yes
MinVersion=10.0

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
ptbr.AssocTask=Registrar o MariPDF para abrir arquivos .pdf
en.AssocTask=Register MariPDF to open .pdf files
ptbr.ChooseDefault=Escolher o MariPDF como leitor padrão de PDF (abre as Configurações do Windows)
en.ChooseDefault=Choose MariPDF as the default PDF reader (opens Windows Settings)
ptbr.Downgrade=Uma versão mais nova do MariPDF (%1) já está instalada.%nDesinstale-a antes de instalar a %2.
en.Downgrade=A newer MariPDF (%1) is already installed.%nUninstall it before installing %2.
ptbr.FileTypeName=Documento PDF
en.FileTypeName=PDF Document
ptbr.AppDescription=Leitor de PDF rápido e focado para Windows.
en.AppDescription=A fast, focused PDF reader for Windows.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "associate"; Description: "{cm:AssocTask}"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; ProgID
Root: HKA; Subkey: "Software\Classes\{#ProgId}"; ValueType: string; ValueName: ""; ValueData: "{cm:FileTypeName}"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\{#ProgId}"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "{cm:FileTypeName}"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\{#ProgId}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\{#ProgId}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate
; "Abrir com"
Root: HKA; Subkey: "Software\Classes\.pdf\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".pdf"; ValueData: ""; Tasks: associate
; Capabilities + RegisteredApplications (aparece em "Aplicativos padrão")
Root: HKA; Subkey: "Software\MariPDF\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#AppName}"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\MariPDF\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{cm:AppDescription}"; Tasks: associate
Root: HKA; Subkey: "Software\MariPDF\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#AppExe},0"; Tasks: associate
Root: HKA; Subkey: "Software\MariPDF\Capabilities\FileAssociations"; ValueType: string; ValueName: ".pdf"; ValueData: "{#ProgId}"; Tasks: associate
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#AppName}"; ValueData: "Software\MariPDF\Capabilities"; Flags: uninsdeletevalue; Tasks: associate
; Remove o MariPDF.Document mesmo se o app o registrou fora do instalador
Root: HKA; Subkey: "Software\MariPDF"; Flags: uninsdeletekeyifempty dontcreatekey

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
Filename: "ms-settings:defaultapps?registeredAppUser={#AppName}"; Description: "{cm:ChooseDefault}"; Flags: shellexec postinstall skipifsilent unchecked; Tasks: associate

[UninstallDelete]
; Estado do usuário (configurações, histórico) é preservado de propósito.
Type: filesandordirs; Name: "{app}"

[Code]
{ Compara versões semânticas: 1.2.3 > 1.2.3-rc.1 > 1.2.2. }
function NextPart(var S: String; Sep: String): String;
var
  P: Integer;
begin
  P := Pos(Sep, S);
  if P = 0 then
  begin
    Result := S;
    S := '';
  end
  else
  begin
    Result := Copy(S, 1, P - 1);
    S := Copy(S, P + 1, Length(S));
  end;
end;

function CompareSemver(A, B: String): Integer;
var
  CoreA, CoreB, PreA, PreB: String;
  I, NA, NB: Integer;
begin
  CoreA := NextPart(A, '-'); PreA := A;
  CoreB := NextPart(B, '-'); PreB := B;
  for I := 1 to 3 do
  begin
    NA := StrToIntDef(NextPart(CoreA, '.'), 0);
    NB := StrToIntDef(NextPart(CoreB, '.'), 0);
    if NA <> NB then
    begin
      if NA > NB then Result := 1 else Result := -1;
      Exit;
    end;
  end;
  if (PreA = '') and (PreB = '') then Result := 0
  else if PreA = '' then Result := 1
  else if PreB = '' then Result := -1
  else Result := CompareText(PreA, PreB);
end;

function InitializeSetup(): Boolean;
var
  Installed: String;
begin
  Result := True;
  if RegQueryStringValue(HKCU,
    'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{#AppIdGuid}}_is1',
    'DisplayVersion', Installed) then
  begin
    if CompareSemver(Installed, '{#AppVersion}') > 0 then
    begin
      SuppressibleMsgBox(FmtMessage(CustomMessage('Downgrade'), [Installed, '{#AppVersion}']),
        mbError, MB_OK, IDOK);
      Result := False;
    end;
  end;
end;
