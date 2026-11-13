; VanityVault's installer: per user, no admin, one exe.
; Built by installer\build.ps1:
;   makensis /DVERSION=0.1.0 /DVERSION4=0.1.0.0 /DEXE=<VanityVault.exe> /DOUTFILE=<setup.exe> app.nsi
;
; Interactive: folder, install, finish (with "Run VanityVault").
; /S /RUN    silent (an updater), then VanityVault starts again.
; The new exe is unpacked next to the old one first and swapped in only once it is complete, so a
; failed or interrupted install leaves the old version working; with /RUN it is started again
; even then, so an update never leaves the user without the app.

Unicode true
SetCompressor /SOLID lzma
RequestExecutionLevel user
ManifestDPIAware true

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

; build.ps1 -Sign only (/DSIGN=<sign.json> /DSIGNED_UNINSTALLER=<copy>): sign.ps1 signs the
; uninstaller before it is packed (WriteUninstaller writes that signed file) and keeps a copy for
; build.ps1's check, then signs the finished installer. Any failure fails the build.
!ifdef SIGN
  !define SIGN_WITH '"$%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "${__FILEDIR__}\sign.ps1" -Config "${SIGN}"'
  !uninstfinalize '${SIGN_WITH} -File "%1" -Keep "${SIGNED_UNINSTALLER}"' = 0
  !finalize '${SIGN_WITH} -File "%1"' = 0
!endif

!define NAME "VanityVault"
!define PUBLISHER "Ted K. Johansen"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${NAME}"

Name "${NAME}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
BrandingText "${NAME} ${VERSION}"

VIProductVersion "${VERSION4}"
VIAddVersionKey "ProductName" "${NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${NAME} installer"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

!define MUI_ICON "..\src-tauri\icons\icon.ico"
!define MUI_UNICON "..\src-tauri\icons\icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${NAME}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Run ${NAME}"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; Started by an updater (/RUN): start VanityVault, whichever version is in place.
Function RestartIfAsked
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/RUN" $R1
  ${IfNot} ${Errors}
  ${AndIf} ${FileExists} "$INSTDIR\${NAME}.exe"
    Exec '"$INSTDIR\${NAME}.exe"'
  ${EndIf}
FunctionEnd

!macro Fail code message
  Delete "$INSTDIR\${NAME}.exe.new"
  DetailPrint "${message}"
  Call RestartIfAsked
  SetErrorLevel ${code}
  Abort "${message}"
!macroend

; Rename $0 to $1, a few tries (an antivirus scan can hold a new file for a moment). Errors set
; when it never worked.
Function RenameRetry
  StrCpy $R3 0
  ${Do}
    ClearErrors
    Rename $0 $1
    ${IfNot} ${Errors}
      Return
    ${EndIf}
    IntOp $R3 $R3 + 1
    Sleep 300
  ${LoopWhile} $R3 < 10
  SetErrors
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"
  SetOverwrite on

  ; Unpack in full first. A running exe can be renamed but not overwritten: the old one steps
  ; aside, the new one takes its name, and the old one goes once nothing runs it any more.
  Delete "$INSTDIR\${NAME}.exe.new"
  ClearErrors
  File "/oname=${NAME}.exe.new" "${EXE}"
  ${If} ${Errors}
    !insertmacro Fail 2 "Could not unpack ${NAME}.exe; the installed version is unchanged."
  ${EndIf}
  ; The last update's old exe may still be running (it is deleted once it is not).
  Delete "$INSTDIR\${NAME}.exe.old"
  Delete "$INSTDIR\${NAME}.exe.old2"
  StrCpy $R9 "$INSTDIR\${NAME}.exe.old"
  ${If} ${FileExists} $R9
    StrCpy $R9 "$INSTDIR\${NAME}.exe.old2"
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\${NAME}.exe"
    ${If} ${FileExists} $R9
      !insertmacro Fail 3 "Older copies of ${NAME} are still running; close them and install again. The installed version is unchanged."
    ${EndIf}
    StrCpy $0 "$INSTDIR\${NAME}.exe"
    StrCpy $1 $R9
    Call RenameRetry
    ${If} ${Errors}
      !insertmacro Fail 3 "${NAME}.exe could not be replaced; the installed version is unchanged."
    ${EndIf}
  ${EndIf}
  StrCpy $0 "$INSTDIR\${NAME}.exe.new"
  StrCpy $1 "$INSTDIR\${NAME}.exe"
  Call RenameRetry
  ${If} ${Errors}
    ; The old one back, as patiently: without it nothing restarts.
    ${If} ${FileExists} $R9
      StrCpy $0 $R9
      StrCpy $1 "$INSTDIR\${NAME}.exe"
      Call RenameRetry
      ${If} ${Errors}
        !insertmacro Fail 4 "Could not put the new ${NAME}.exe in place, nor the old one back ($R9); install again."
      ${EndIf}
    ${EndIf}
    !insertmacro Fail 4 "Could not put the new ${NAME}.exe in place; the installed version is unchanged."
  ${EndIf}
  Delete $R9 ; still running: it goes with the next install

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\${NAME}.exe"

  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${NAME}.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0

  Call RestartIfAsked
SectionEnd

; The vault (%LOCALAPPDATA%\io.vanityvault.app) is never touched: it holds the user's keys, and a
; reinstall finds it where it was. The uninstaller says so.
Section "Uninstall"
  ; Older copies first: if one still runs, nothing else has been touched yet. ($R6, not the
  ; error flag across checks: LogicLib's ${Errors} test clears it.)
  StrCpy $R6 0
  ClearErrors
  Delete "$INSTDIR\${NAME}.exe.old"
  Delete "$INSTDIR\${NAME}.exe.old2"
  IfErrors 0 +2
    StrCpy $R6 1
  ${If} $R6 == 0
    ClearErrors
    Delete "$INSTDIR\${NAME}.exe"
    IfErrors 0 +2
      StrCpy $R6 1
  ${EndIf}
  ${If} $R6 == 1
    MessageBox MB_OK|MB_ICONEXCLAMATION "${NAME} is running. Close it, then uninstall again." /SD IDOK
    SetErrorLevel 3
    Abort
  ${EndIf}
  Delete "$INSTDIR\${NAME}.exe.new"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\${NAME}.lnk"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  ${If} ${FileExists} "$LOCALAPPDATA\io.vanityvault.app\vault.json"
    DetailPrint "Your vault was kept in $LOCALAPPDATA\io.vanityvault.app."
    MessageBox MB_OK|MB_ICONINFORMATION "${NAME} is uninstalled. Your vault, with every key you found, was kept in $LOCALAPPDATA\io.vanityvault.app. Reinstall to open it again. Only delete that folder if you have your keys somewhere else." /SD IDOK
  ${EndIf}
SectionEnd
