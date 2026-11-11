; curado installer for Windows.
;
; Based on Tauri's NSIS template (tauri-cli 2.12.4, tauri-bundler installer.nsi). Blocks
; marked "curado:" differ from upstream; the rest is upstream, so a Tauri upgrade is a
; three-way merge of this file. The theme comes from installer-kit.nsh (app-agnostic) and
; palette.nsh (generated). Design, voice and contract: docs/INSTALLER.md.
Unicode true
ManifestDPIAware true
; Add in `dpiAwareness` `PerMonitorV2` to manifest for Windows 10 1607+ (note this should not affect lower versions since they should be able to ignore this and pick up `dpiAware` `true` set by `ManifestDPIAware true`)
; Currently undocumented on NSIS's website but is in the Docs folder of source tree, see
; https://github.com/kichik/nsis/blob/5fc0b87b819a9eec006df4967d08e522ddd651c9/Docs/src/attributes.but#L286-L300
; https://github.com/tauri-apps/tauri/pull/10106
ManifestDPIAwareness PerMonitorV2

!if "{{compression}}" == "none"
  SetCompress off
!else
  ; Set the compression algorithm. We default to LZMA.
  SetCompressor /SOLID "{{compression}}"
!endif

; Keep above !include to stay ahead of any plugin command
; see https://github.com/tauri-apps/tauri/pull/15422#discussion_r3289239624
{{#if signed_plugins_path}}
!addplugindir "{{signed_plugins_path}}"
{{/if}}

!include MUI2.nsh
!include FileFunc.nsh
!include x64.nsh
!include WordFunc.nsh
!include "utils.nsh"
!include "FileAssociation.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "Win\RestartManager.nsh"
!include "StrFunc.nsh"
${StrCase}
${StrLoc}

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

; curado: the theme kit arrives through the hooks slot
!ifndef NU_KIT_DIR
  !error "curado: set bundle.windows.nsis.installerHooks to windows/installer-kit.nsh"
!endif

!define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

!define MANUFACTURER "{{manufacturer}}"
!define PRODUCTNAME "{{product_name}}"
!define VERSION "{{version}}"
!define VERSIONWITHBUILD "{{version_with_build}}"
!define HOMEPAGE "{{homepage}}"
!define INSTALLMODE "{{install_mode}}"
!define LICENSE "{{license}}"
!define INSTALLERICON "{{installer_icon}}"
!define SIDEBARIMAGE "{{sidebar_image}}"
!define HEADERIMAGE "{{header_image}}"
!define UNINSTALLERICON "{{uninstaller_icon}}"
!define UNINSTALLERHEADERIMAGE "{{uninstaller_header_image}}"
!define MAINBINARYNAME "{{main_binary_name}}"
!define MAINBINARYSRCPATH "{{main_binary_path}}"
!define BUNDLEID "{{bundle_id}}"
!define COPYRIGHT "{{copyright}}"
!define OUTFILE "{{out_file}}"
!define ARCH "{{arch}}"
!define ADDITIONALPLUGINSPATH "{{additional_plugins_path}}"
!define ALLOWDOWNGRADES "{{allow_downgrades}}"
!define DISPLAYLANGUAGESELECTOR "{{display_language_selector}}"
!define INSTALLWEBVIEW2MODE "{{install_webview2_mode}}"
!define WEBVIEW2INSTALLERARGS "{{webview2_installer_args}}"
!define WEBVIEW2BOOTSTRAPPERPATH "{{webview2_bootstrapper_path}}"
!define WEBVIEW2INSTALLERPATH "{{webview2_installer_path}}"
!define MINIMUMWEBVIEW2VERSION "{{minimum_webview2_version}}"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define UNINSTALLERSIGNCOMMAND "{{uninstaller_sign_cmd}}"
!define ESTIMATEDSIZE "{{estimated_size}}"
!define STARTMENUFOLDER "{{start_menu_folder}}"

Var PassiveMode
Var UpdateMode
Var NoShortcutMode
Var WixMode
Var OldMainBinaryName

; curado: pages
Var CuradoDialog
Var CuradoImage
Var CuradoImageHandle
Var CuradoFolder
Var CuradoInstalled
Var CuradoState
Var CuradoRun
Var CuradoDesktop
Var CuradoDemoTag

Name "${PRODUCTNAME}"
BrandingText "${COPYRIGHT}"
; curado: the window says what it does, and progress goes straight to the last page.
; A failed run stays on the progress page with its details.
Caption "$(curadoCaption)$CuradoDemoTag"
UninstallCaption "$(curadoUnCaption)"
SubCaption 3 " "
SubCaption 4 " "
UninstallSubCaption 0 " "
UninstallSubCaption 1 " "
UninstallSubCaption 2 " "
AutoCloseWindow true
OutFile "${OUTFILE}"

; We don't actually use this value as default install path,
; it's just for nsis to append the product name folder in the directory selector
; https://nsis.sourceforge.io/Reference/InstallDir
!define PLACEHOLDER_INSTALL_DIR "placeholder\${PRODUCTNAME}"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"

VIProductVersion "${VERSIONWITHBUILD}"
VIAddVersionKey "ProductName" "${PRODUCTNAME}"
VIAddVersionKey "CompanyName" "${MANUFACTURER}" ; curado: who publishes it
VIAddVersionKey "FileDescription" "${PRODUCTNAME}"
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"

# additional plugins
!addplugindir "${ADDITIONALPLUGINSPATH}"

; Uninstaller signing command
!if "${UNINSTALLERSIGNCOMMAND}" != ""
  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}'
!endif

; Handle install mode, `perUser`, `perMachine` or `both`
!if "${INSTALLMODE}" == "perMachine"
  RequestExecutionLevel admin
!endif

!if "${INSTALLMODE}" == "currentUser"
  RequestExecutionLevel user
!endif

!if "${INSTALLMODE}" == "both"
  !define MULTIUSER_MUI
  !define MULTIUSER_INSTALLMODE_INSTDIR "${PRODUCTNAME}"
  !define MULTIUSER_INSTALLMODE_COMMANDLINE
  !if "${ARCH}" == "x64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !else if "${ARCH}" == "arm64"
    !define MULTIUSER_USE_PROGRAMFILES64
  !endif
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_KEY "${UNINSTKEY}"
  !define MULTIUSER_INSTALLMODE_DEFAULT_REGISTRY_VALUENAME "CurrentUser"
  !define MULTIUSER_INSTALLMODEPAGE_SHOWUSERNAME
  !define MULTIUSER_INSTALLMODE_FUNCTION RestorePreviousInstallLocation
  !define MULTIUSER_EXECUTIONLEVEL Highest
  !include MultiUser.nsh
!endif

; Installer icon
!if "${INSTALLERICON}" != ""
  !define MUI_ICON "${INSTALLERICON}"
!endif

; Installer sidebar image
!if "${SIDEBARIMAGE}" != ""
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBARIMAGE}"
!endif

; Enable header images for installer and uninstaller pages when either image is configured.
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!else if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE
!endif

; Installer header image
!if "${HEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_BITMAP "${HEADERIMAGE}"
  !define MUI_HEADERIMAGE_RIGHT ; curado: text first, mark at the right
!endif

; curado: theme the outer dialog as soon as it exists
!define MUI_CUSTOMFUNCTION_GUIINIT CuradoGuiInit
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.CuradoGuiInit

; Uninstaller header image
!if "${UNINSTALLERHEADERIMAGE}" != ""
  !define MUI_HEADERIMAGE_UNBITMAP "${UNINSTALLERHEADERIMAGE}"
!endif

; Uninstaller icon
!if "${UNINSTALLERICON}" != ""
  !define MUI_UNICON "${UNINSTALLERICON}"
!endif

; Define registry key to store installer language
!define MUI_LANGDLL_REGISTRY_ROOT "HKCU"
!define MUI_LANGDLL_REGISTRY_KEY "${MANUPRODUCTKEY}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "Installer Language"

; Installer pages, must be ordered as they appear
; 1. Welcome page
; curado: one page says what, where and which version, and its button installs.
Page custom CuradoWelcome CuradoWelcomeLeave

; 2. License Page (if defined)
!if "${LICENSE}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MUI_PAGE_LICENSE "${LICENSE}"
!endif

; 3. Install mode (if it is set to `both`)
!if "${INSTALLMODE}" == "both"
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !insertmacro MULTIUSER_PAGE_INSTALLMODE
!endif

; 4. Custom page to ask user if he wants to reinstall/uninstall
;    only if a previous installation was detected
Var ReinstallPageCheck
Page custom PageReinstall PageLeaveReinstall

; Find a previous WiX installation and set $R6 to its uninstall registry key, or to "" if none.
;
; A WiX installer stores the installation info in registry
; using a UUID and so we have to loop through all keys under
; `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
; and check if `DisplayName` and `Publisher` keys match ${PRODUCTNAME} and ${MANUFACTURER}
;
; This has a potential issue that there maybe another installation that matches
; our ${PRODUCTNAME} and ${MANUFACTURER} but wasn't installed by our WiX installer,
; however, this should be fine since the user will have to confirm the uninstallation
; and they can chose to abort it if doesn't make sense.
Function FindWixInstall
  StrCpy $R6 ""
  StrCpy $0 0
  wix_loop:
    EnumRegKey $1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" wix_loop_done ; Exit loop if there is no more keys to loop on
    IntOp $0 $0 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "Publisher"
    StrCmp "$R0$R1" "${PRODUCTNAME}${MANUFACTURER}" 0 wix_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "UninstallString"
    ${StrCase} $R1 $R0 "L"
    ${StrLoc} $R0 $R1 "msiexec" ">"
    StrCmp $R0 0 0 wix_loop_done
    StrCpy $R6 "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1"
  wix_loop_done:
FunctionEnd

Function PageReinstall
  ; curado: /DEMO has no maintenance step
  ${If} $NuDemo = 1
    Abort
  ${EndIf}

  ; Uninstall previous WiX installation if exists.
  Call FindWixInstall
  ${If} $R6 != ""
    StrCpy $WixMode 1
    Goto compare_version
  ${EndIf}

  ; Check if there is an existing installation, if not, abort the reinstall page
  ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
  ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
  ${IfThen} "$R0$R1" == "" ${|} Abort ${|}

  ; Compare this installar version with the existing installation
  ; and modify the messages presented to the user accordingly
  compare_version:
  StrCpy $R4 "$(older)"
  ${If} $WixMode = 1
    ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  ${IfThen} $R0 == "" ${|} StrCpy $R4 "$(unknown)" ${|}

  nsis_tauri_utils::SemverCompare "${VERSION}" $R0
  Pop $R0

  ; curado: an upgrade installs over the old version without asking, as /UPDATE and
  ; passive mode already do. The welcome page said "from x to y" and that the library stays.
  ${If} $R0 = 1
  ${AndIf} $WixMode <> 1
    Abort
  ${EndIf}

  ; Read the install choice header through $R5 so a custom language file that only
  ; defines the misspelled `choowHowToInstall` still shows its own text
  ; TODO: remove the `choowHowToInstall` fallback in a future major version
  StrCpy $R5 "$(chooseHowToInstall)"
  ${IfThen} $R5 == "" ${|} StrCpy $R5 "$(choowHowToInstall)" ${|}

  ; Reinstalling the same version
  ${If} $R0 = 0
    ; curado: its own words for the one maintenance case most people meet
    StrCpy $R1 "$(curadoSameLong)"
    StrCpy $R2 "$(curadoReinstall)"
    StrCpy $R3 "$(curadoRemove)"
    !insertmacro MUI_HEADER_TEXT "$(curadoSameTitle)" "$(curadoSameSub)"
  ; Upgrading
  ${ElseIf} $R0 = 1
    StrCpy $R1 "$(olderOrUnknownVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    StrCpy $R3 "$(dontUninstall)"
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$R5"
  ; Downgrading
  ${ElseIf} $R0 = -1
    StrCpy $R1 "$(newerVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    !if "${ALLOWDOWNGRADES}" == "true"
      StrCpy $R3 "$(dontUninstall)"
    !else
      StrCpy $R3 "$(dontUninstallDowngrade)"
    !endif
    !insertmacro MUI_HEADER_TEXT "$(alreadyInstalled)" "$R5"
  ${Else}
    Abort
  ${EndIf}

  ; Skip showing the page if passive
  ;
  ; Note that we don't call this earlier at the beginning
  ; of this function because we need to populate some variables
  ; related to current installed version if detected and whether
  ; we are downgrading or not.
  ${If} $PassiveMode = 1
    Call PageLeaveReinstall
  ${Else}
    nsDialogs::Create 1018
    Pop $R4
    ${IfThen} $(^RTL) = 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}

    ${NSD_CreateLabel} 0 0 100% 24u $R1
    Pop $R1

    ${NSD_CreateRadioButton} 30u 50u -30u 8u $R2
    Pop $R2
    ${NSD_OnClick} $R2 PageReinstallUpdateSelection

    ${NSD_CreateRadioButton} 30u 70u -30u 8u $R3
    Pop $R3
    ; Disable this radio button if downgrading and downgrades are disabled
    !if "${ALLOWDOWNGRADES}" == "false"
      ${IfThen} $R0 = -1 ${|} EnableWindow $R3 0 ${|}
    !endif
    ${NSD_OnClick} $R3 PageReinstallUpdateSelection

    ; Check the first radio button if this the first time
    ; we enter this page or if the second button wasn't
    ; selected the last time we were on this page
    ${If} $ReinstallPageCheck <> 2
      SendMessage $R2 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${Else}
      SendMessage $R3 ${BM_SETCHECK} ${BST_CHECKED} 0
    ${EndIf}

    ; curado: theme. Themed radio buttons ignore their text color, classic ones honor it.
    !insertmacro NU_COLOR $R4 "${NU_TEXT}" "${NU_SHEET}"
    !insertmacro NU_COLOR $R1 "${NU_TEXT}" "${NU_SHEET}"
    !insertmacro NU_CLASSIC $R2
    !insertmacro NU_COLOR $R2 "${NU_TEXT}" "${NU_SHEET}"
    !insertmacro NU_CLASSIC $R3
    !insertmacro NU_COLOR $R3 "${NU_TEXT}" "${NU_SHEET}"
    SendMessage $mui.Button.Next ${WM_SETTEXT} 0 "STR:$(curadoNext)"
    ShowWindow $mui.Button.Back ${SW_HIDE}

    ${NSD_SetFocus} $R2
    nsDialogs::Show
  ${EndIf}
FunctionEnd
Function PageReinstallUpdateSelection
  ${NSD_GetState} $R2 $R1
  ${If} $R1 == ${BST_CHECKED}
    StrCpy $ReinstallPageCheck 1
  ${Else}
    StrCpy $ReinstallPageCheck 2
  ${EndIf}
FunctionEnd
Function PageLeaveReinstall
  ${NSD_GetState} $R2 $R1

  ; If migrating from Wix, always uninstall
  ${If} $WixMode = 1
    Goto reinst_uninstall
  ${EndIf}

  ; In update mode, always proceeds without uninstalling
  ${If} $UpdateMode = 1
    Goto reinst_done
  ${EndIf}

  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
  ; $R1 holds the radio buttons state:
  ;   1 => first choice was selected
  ;   0 => second choice was selected
  ${If} $R0 = 0 ; Same version, proceed
    ${If} $R1 = 1              ; User chose to add/reinstall
      Goto reinst_done
    ${Else}                    ; User chose to uninstall
      Goto reinst_uninstall
    ${EndIf}
  ${ElseIf} $R0 = 1 ; Upgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${ElseIf} $R0 = -1 ; Downgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${EndIf}

  reinst_uninstall:
    HideWindow
    ClearErrors

    ${If} $WixMode = 1
      ReadRegStr $R1 HKLM "$R6" "UninstallString"
      ExecWait '$R1' $0
    ${Else}
      ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
      ; curado: an install from before the publisher changed is found through $INSTDIR
      ${IfThen} $4 == "" ${|} StrCpy $4 $INSTDIR ${|}
      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE
      ${IfThen} $PassiveMode = 1 ${|} StrCpy $R1 "$R1 /P" ${|} ; append /P
      StrCpy $R1 "$R1 _?=$4" ; append uninstall directory
      ExecWait '$R1' $0
    ${EndIf}

    BringToFront

    ${IfThen} ${Errors} ${|} StrCpy $0 2 ${|} ; ExecWait failed, set fake exit code

    ${If} $0 <> 0
    ${OrIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ; User cancelled wix uninstaller? return to select un/reinstall page
      ${If} $WixMode = 1
      ${AndIf} $0 = 1602
        Abort
      ${EndIf}

      ; User cancelled NSIS uninstaller? return to select un/reinstall page
      ${If} $0 = 1
        Abort
      ${EndIf}

      ; Other errors? show generic error message and return to select un/reinstall page
      MessageBox MB_ICONEXCLAMATION "$(unableToUninstall)"
      Abort
    ${EndIf}

    ; The NSIS uninstaller was run in place (`_?=`), so it cannot delete itself
    ; and the install directory, clean them up here once the uninstall succeeded
    ; https://nsis.sourceforge.io/Docs/Chapter3.html#:~:text=path%20contains%20spaces.-,3.2.3%20Examples,-installer.exe%20/NCRC
    ${If} $WixMode = 0
    ${AndIf} $4 != ""
      Delete "$4\uninstall.exe"
      RMDir "$4"
    ${EndIf}

    ; curado: on the same version, "uninstall" removes the app and ends here.
    ; Upstream goes on to install it again, which is not what the choice says.
    ${If} $R0 = 0
    ${AndIf} $WixMode <> 1
      Quit
    ${EndIf}
  reinst_done:
FunctionEnd

; 5. Choose install directory page
; curado: none. The welcome page shows the folder and can change it; /D= still works.

; 6. Start menu shortcut page
Var AppStartMenuFolder
!if "${STARTMENUFOLDER}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !define MUI_STARTMENUPAGE_DEFAULTFOLDER "${STARTMENUFOLDER}"
!else
  !define MUI_PAGE_CUSTOMFUNCTION_PRE Skip
!endif
!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder

; 7. Installation page
; curado: themed, with its own words
!define MUI_PAGE_HEADER_TEXT "$(curadoWorking)"
!define MUI_PAGE_HEADER_SUBTEXT "$(curadoWorkingSub)"
!define MUI_INSTFILESPAGE_FINISHHEADER_TEXT "$(curadoDone)"
!define MUI_INSTFILESPAGE_FINISHHEADER_SUBTEXT "$(curadoDoneText)"
!define MUI_INSTFILESPAGE_ABORTHEADER_TEXT "$(curadoFailed)"
!define MUI_INSTFILESPAGE_ABORTHEADER_SUBTEXT "$(curadoFailedSub)"
!define MUI_PAGE_CUSTOMFUNCTION_SHOW CuradoProgressShow
!insertmacro MUI_PAGE_INSTFILES

; 8. Finish page
; curado: a themed page with upstream's two choices (run the app, desktop shortcut),
; which call the same functions as upstream's finish page.
Page custom CuradoFinish CuradoFinishLeave

Function RunMainBinary
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
FunctionEnd

; Uninstaller Pages
; 1. Confirm uninstall page
Var DeleteAppDataCheckbox
Var DeleteAppDataCheckboxState
!define /ifndef WS_EX_LAYOUTRTL         0x00400000
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow
Function un.ConfirmShow ; Add add a `Delete app data` check box
  ; $1 inner dialog HWND
  ; $2 window DPI
  ; $3 style
  ; $4 x
  ; $5 y
  ; $6 width
  ; $7 height
  FindWindow $1 "#32770" "" $HWNDPARENT ; Find inner dialog
  System::Call "user32::GetDpiForWindow(p r1) i .r2"
  ${If} $(^RTL) = 1
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE} | ${WS_EX_LAYOUTRTL}"
    IntOp $4 50 * $2
  ${Else}
    StrCpy $3 "${__NSD_CheckBox_EXSTYLE}"
    IntOp $4 0 * $2
  ${EndIf}
  IntOp $5 100 * $2
  IntOp $6 400 * $2
  IntOp $7 25 * $2
  IntOp $4 $4 / 96
  IntOp $5 $5 / 96
  IntOp $6 $6 / 96
  IntOp $7 $7 / 96
  System::Call 'user32::CreateWindowEx(i r3, w "${__NSD_CheckBox_CLASS}", w "$(curadoDeleteData)", i ${__NSD_CheckBox_STYLE}, i r4, i r5, i r6, i r7, p r1, i0, i0, i0) i .s'
  Pop $DeleteAppDataCheckbox
  SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $1
  SendMessage $DeleteAppDataCheckbox ${WM_SETFONT} $1 1

  ; curado: theme the page and the check box
  !insertmacro NU_INNER_PAGE
  !insertmacro NU_INNER_TEXT 1006 "${NU_TEXT}"
  !insertmacro NU_INNER_TEXT 1029 "${NU_TEXT_2}"
  !insertmacro NU_INNER_TEXT 1000 "${NU_TEXT}"
  !insertmacro NU_CLASSIC $DeleteAppDataCheckbox
  !insertmacro NU_COLOR $DeleteAppDataCheckbox "${NU_TEXT}" "${NU_SHEET}"
  ShowWindow $mui.Button.Back ${SW_HIDE}
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave
Function un.ConfirmLeave
  SendMessage $DeleteAppDataCheckbox ${BM_GETCHECK} 0 0 $DeleteAppDataCheckboxState
FunctionEnd
!define MUI_PAGE_CUSTOMFUNCTION_PRE un.SkipIfPassive
; curado: says what stays before anything is removed
!define MUI_PAGE_HEADER_TEXT "$(curadoUnTitle)"
!define MUI_PAGE_HEADER_SUBTEXT "$(curadoUnSub)"
!define MUI_UNCONFIRMPAGE_TEXT_TOP "$(curadoUnTop)"
!insertmacro MUI_UNPAGE_CONFIRM

; 2. Uninstalling Page
; curado: themed, with its own words
; Modern UI never unsets the abort header of the install page, so drop it here
!undef MUI_INSTFILESPAGE_ABORTHEADER_TEXT
!undef MUI_INSTFILESPAGE_ABORTHEADER_SUBTEXT
!define MUI_PAGE_HEADER_TEXT "$(curadoUnWorking)"
!define MUI_PAGE_HEADER_SUBTEXT "$(curadoUnWorkingSub)"
!define MUI_INSTFILESPAGE_FINISHHEADER_TEXT "$(curadoUnDone)"
!define MUI_INSTFILESPAGE_FINISHHEADER_SUBTEXT "$(curadoUnDoneSub)"
!define MUI_INSTFILESPAGE_ABORTHEADER_TEXT "$(curadoUnFailed)"
!define MUI_INSTFILESPAGE_ABORTHEADER_SUBTEXT "$(curadoFailedSub)"
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.CuradoProgressShow
!insertmacro MUI_UNPAGE_INSTFILES

;Languages
; PageReinstall reads the misspelled `choowHowToInstall` as a fallback, and makensis warns
; about a language string that is read but never set, so default it to an empty string for
; every language. This runs before the language files are included, so a custom language
; file that only defines the old spelling overrides the default and the fallback shows its
; text. `0` as the language id means the language inserted right above.
; TODO: remove the `choowHowToInstall` fallback in a future major version
{{#each languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
LangString choowHowToInstall 0 ""
{{/each}}
!insertmacro MUI_RESERVEFILE_LANGDLL
{{#each language_files}}
  !include "{{this}}"
{{/each}}

; ---------------------------------------------------------------------------
; curado: words. Spanish is the app's base language and the fallback; English is used
; on an English system. Clear first, playful second: a button always says what it does,
; and every fact (what, where, your data, permissions) is stated plainly.

!ifdef LANG_SPANISH
LangString curadoCaption ${LANG_SPANISH} "Instalar ${PRODUCTNAME}"
LangString curadoUnCaption ${LANG_SPANISH} "Desinstalar ${PRODUCTNAME}"
LangString curadoHello ${LANG_SPANISH} "Pásale a lo barrido."
LangString curadoHelloUpdate ${LANG_SPANISH} "¡Quihubo! Llegó la nueva."
LangString curadoHelloSame ${LANG_SPANISH} "Ya te la sabes."
LangString curadoTagline ${LANG_SPANISH} "Tu música. Dos decks. Espacio para jugar."
LangString curadoWhat ${LANG_SPANISH} "QUÉ"
LangString curadoWhere ${LANG_SPANISH} "DÓNDE"
LangString curadoMusic ${LANG_SPANISH} "TU MÚSICA"
LangString curadoAdmin ${LANG_SPANISH} "PERMISOS"
LangString curadoWhatFresh ${LANG_SPANISH} "${PRODUCTNAME} ${VERSION} para Windows (${ARCH})."
LangString curadoWhatUpdate ${LANG_SPANISH} "De la $CuradoInstalled a la ${VERSION}. Tu biblioteca se queda."
LangString curadoWhatSame ${LANG_SPANISH} "Ya tienes la ${VERSION}. Puedes reinstalarla o quitarla."
LangString curadoWhatOlder ${LANG_SPANISH} "Tienes la $CuradoInstalled, más nueva que esta ${VERSION}."
LangString curadoMusicText ${LANG_SPANISH} "La toco, pero no la muevo ni la edito."
LangString curadoAdminText ${LANG_SPANISH} "Sin administrador. Sin letras chiquitas."
LangString curadoChange ${LANG_SPANISH} "Cambiar…"
LangString curadoPickFolder ${LANG_SPANISH} "¿Dónde ponemos ${PRODUCTNAME}?"
LangString curadoCredits ${LANG_SPANISH} "Hecho por nu9ve. Licencias: carpeta «licenses»."
LangString curadoDemo ${LANG_SPANISH} "DEMO: recorre el instalador y no instala nada."
LangString curadoInstall ${LANG_SPANISH} "Instalar"
LangString curadoUpdate ${LANG_SPANISH} "Actualizar"
LangString curadoNext ${LANG_SPANISH} "Siguiente"
LangString curadoNotNow ${LANG_SPANISH} "Ahorita no"
LangString curadoPlay ${LANG_SPANISH} "¡A darle!"
LangString curadoRunning ${LANG_SPANISH} "${PRODUCTNAME} está abierto, y a lo mejor sonando.$\n$\nCiérralo cuando acabe la rola y le seguimos. El instalador no lo cierra por ti."
LangString curadoCannotWrite ${LANG_SPANISH} "No puedo escribir en esa carpeta.$\n$\nElige otra con «Cambiar…»."
LangString curadoWorking ${LANG_SPANISH} "Acomodando las bocinas…"
LangString curadoWorkingSub ${LANG_SPANISH} "Es rápido: no te me desesperes."
LangString curadoDone ${LANG_SPANISH} "¡Ya quedó!"
LangString curadoDoneText ${LANG_SPANISH} "Ya vive en tu menú Inicio."
LangString curadoFailed ${LANG_SPANISH} "Algo tronó"
LangString curadoFailedSub ${LANG_SPANISH} "No terminó. El detalle dice dónde; vuelve a intentar."
LangString curadoDemoStep ${LANG_SPANISH} "Demo: aquí se copiarían los archivos."
LangString curadoDemoDone ${LANG_SPANISH} "Fin de la demo: no se instaló nada."
LangString curadoFirst ${LANG_SPANISH} "PRIMER TOQUE"
LangString curadoFirstText ${LANG_SPANISH} "Abre «sesión de prueba», dale play con F y J, y mueve el crossfader."
LangString curadoRunNow ${LANG_SPANISH} "Abrir ${PRODUCTNAME}"
LangString curadoDesktop ${LANG_SPANISH} "Dejar un acceso directo en el escritorio"
LangString curadoSameTitle ${LANG_SPANISH} "Ya está instalado"
LangString curadoSameSub ${LANG_SPANISH} "Es la misma versión. ¿Qué hacemos?"
LangString curadoSameLong ${LANG_SPANISH} "Ya tienes ${PRODUCTNAME} ${VERSION}. Tu biblioteca no se toca en ningún caso."
LangString curadoReinstall ${LANG_SPANISH} "Reinstalar (lo deja como nuevo)"
LangString curadoRemove ${LANG_SPANISH} "Desinstalar ${PRODUCTNAME}"
LangString curadoUnTitle ${LANG_SPANISH} "¿Ya te vas?"
LangString curadoUnSub ${LANG_SPANISH} "Va, sin rencor. Tu música ni se entera."
LangString curadoUnTop ${LANG_SPANISH} "Se quita ${PRODUCTNAME} de esta carpeta. Tu música se queda donde está, y tu biblioteca también, salvo que marques la casilla."
LangString curadoDeleteData ${LANG_SPANISH} "Borrar también mi biblioteca, ajustes y stems generados"
LangString curadoUnWorking ${LANG_SPANISH} "Recogiendo los cables…"
LangString curadoUnWorkingSub ${LANG_SPANISH} "Un momento."
LangString curadoUnDone ${LANG_SPANISH} "Listo. Ahí nos vidrios."
LangString curadoUnDoneSub ${LANG_SPANISH} "${PRODUCTNAME} se fue; tu música sigue donde estaba."
LangString curadoUnFailed ${LANG_SPANISH} "No se pudo quitar todo"
!endif

!ifdef LANG_ENGLISH
LangString curadoCaption ${LANG_ENGLISH} "Install ${PRODUCTNAME}"
LangString curadoUnCaption ${LANG_ENGLISH} "Uninstall ${PRODUCTNAME}"
LangString curadoHello ${LANG_ENGLISH} "Come on in."
LangString curadoHelloUpdate ${LANG_ENGLISH} "Back for the new one."
LangString curadoHelloSame ${LANG_ENGLISH} "You know the drill."
LangString curadoTagline ${LANG_ENGLISH} "Your music. Two decks. Room to play."
LangString curadoWhat ${LANG_ENGLISH} "WHAT"
LangString curadoWhere ${LANG_ENGLISH} "WHERE"
LangString curadoMusic ${LANG_ENGLISH} "YOUR MUSIC"
LangString curadoAdmin ${LANG_ENGLISH} "ADMIN"
LangString curadoWhatFresh ${LANG_ENGLISH} "${PRODUCTNAME} ${VERSION} for Windows (${ARCH})."
LangString curadoWhatUpdate ${LANG_ENGLISH} "From $CuradoInstalled to ${VERSION}. Your library stays."
LangString curadoWhatSame ${LANG_ENGLISH} "You already have ${VERSION}. Reinstall it or remove it."
LangString curadoWhatOlder ${LANG_ENGLISH} "You have $CuradoInstalled, newer than this ${VERSION}."
LangString curadoMusicText ${LANG_ENGLISH} "I play it. I never move or edit it."
LangString curadoAdminText ${LANG_ENGLISH} "Not needed. No fine print."
LangString curadoChange ${LANG_ENGLISH} "Change…"
LangString curadoPickFolder ${LANG_ENGLISH} "Where should ${PRODUCTNAME} live?"
LangString curadoCredits ${LANG_ENGLISH} "Made by nu9ve. Licenses: the licenses folder."
LangString curadoDemo ${LANG_ENGLISH} "DEMO: walks the installer and installs nothing."
LangString curadoInstall ${LANG_ENGLISH} "Install"
LangString curadoUpdate ${LANG_ENGLISH} "Update"
LangString curadoNext ${LANG_ENGLISH} "Next"
LangString curadoNotNow ${LANG_ENGLISH} "Not now"
LangString curadoPlay ${LANG_ENGLISH} "Let's play"
LangString curadoRunning ${LANG_ENGLISH} "${PRODUCTNAME} is open, and maybe playing.$\n$\nClose it when the track ends and we will carry on. The installer will not close it for you."
LangString curadoCannotWrite ${LANG_ENGLISH} "I cannot write to that folder.$\n$\nPick another one with Change…"
LangString curadoWorking ${LANG_ENGLISH} "Wiring up the speakers…"
LangString curadoWorkingSub ${LANG_ENGLISH} "This is quick."
LangString curadoDone ${LANG_ENGLISH} "All set!"
LangString curadoDoneText ${LANG_ENGLISH} "Find it in your Start menu."
LangString curadoFailed ${LANG_ENGLISH} "Something blew a fuse"
LangString curadoFailedSub ${LANG_ENGLISH} "It did not finish. The details say where; try again."
LangString curadoDemoStep ${LANG_ENGLISH} "Demo: files would be copied here."
LangString curadoDemoDone ${LANG_ENGLISH} "End of the demo: nothing was installed."
LangString curadoFirst ${LANG_ENGLISH} "FIRST MIX"
LangString curadoFirstText ${LANG_ENGLISH} "Open «sesión de prueba», press F and J to play, and move the crossfader."
LangString curadoRunNow ${LANG_ENGLISH} "Open ${PRODUCTNAME}"
LangString curadoDesktop ${LANG_ENGLISH} "Leave a shortcut on the desktop"
LangString curadoSameTitle ${LANG_ENGLISH} "Already installed"
LangString curadoSameSub ${LANG_ENGLISH} "Same version. What now?"
LangString curadoSameLong ${LANG_ENGLISH} "You already have ${PRODUCTNAME} ${VERSION}. Your library is not touched either way."
LangString curadoReinstall ${LANG_ENGLISH} "Reinstall (a fresh copy)"
LangString curadoRemove ${LANG_ENGLISH} "Uninstall ${PRODUCTNAME}"
LangString curadoUnTitle ${LANG_ENGLISH} "Leaving already?"
LangString curadoUnSub ${LANG_ENGLISH} "No hard feelings. Your music will not even notice."
LangString curadoUnTop ${LANG_ENGLISH} "This removes ${PRODUCTNAME} from this folder. Your music stays where it is, and so does your library unless you tick the box."
LangString curadoDeleteData ${LANG_ENGLISH} "Also delete my library, settings and generated stems"
LangString curadoUnWorking ${LANG_ENGLISH} "Packing up the cables…"
LangString curadoUnWorkingSub ${LANG_ENGLISH} "One moment."
LangString curadoUnDone ${LANG_ENGLISH} "Done. See you around."
LangString curadoUnDoneSub ${LANG_ENGLISH} "${PRODUCTNAME} is gone; your music is where it was."
LangString curadoUnFailed ${LANG_ENGLISH} "Could not remove everything"
!endif

; ---------------------------------------------------------------------------
; curado: theme and pages. Colors, fonts and window chrome come from installer-kit.nsh.

!macro CURADO_SHARED UN
Function ${UN}CuradoGuiInit
  !insertmacro NU_LOAD_FONTS
  !insertmacro NU_OUTER_DIALOG
  !if "${UN}" == ""
    !if "${SIDEBARIMAGE}" != ""
      File "/oname=$PLUGINSDIR\sidebar.bmp" "${SIDEBARIMAGE}"
    !endif
  !endif
FunctionEnd

Function ${UN}CuradoProgressShow
  !insertmacro NU_STYLE_PROGRESS_PAGE
  ShowWindow $mui.Button.Back ${SW_HIDE}
FunctionEnd
!macroend
!insertmacro CURADO_SHARED ""
!insertmacro CURADO_SHARED "un."

; A page over the whole window: the art at the left, the sheet at the right.
!macro CURADO_FULL_PAGE
  nsDialogs::Create 1044
  Pop $CuradoDialog
  ${If} $CuradoDialog == error
    Abort
  ${EndIf}
  !insertmacro NU_COLOR $CuradoDialog "${NU_TEXT}" "${NU_SHEET}"
  ${NSD_CreateBitmap} 0u 0u 109u 193u ""
  Pop $CuradoImage
  ${NSD_SetStretchedImage} $CuradoImage "$PLUGINSDIR\sidebar.bmp" $CuradoImageHandle
!macroend

!macro CURADO_TEXT X Y W H TEXT FONT COLOR
  ${NSD_CreateLabel} ${X}u ${Y}u ${W}u ${H}u "${TEXT}"
  Pop $0
  !if "${FONT}" != ""
    SendMessage $0 ${WM_SETFONT} ${FONT} 0
  !endif
  !insertmacro NU_COLOR $0 "${COLOR}" "${NU_SHEET}"
!macroend

; One fact: a small label in the accent, and what it says.
!macro CURADO_FACT Y HEIGHT LABEL VALUE
  !insertmacro CURADO_TEXT 124 ${Y} 46 10 "${LABEL}" $NuFontLabel "${NU_ACCENT_TEXT}"
  !insertmacro CURADO_TEXT 172 ${Y} 147 ${HEIGHT} "${VALUE}" "" "${NU_TEXT}"
!macroend

Function CuradoWelcome
  Call SkipIfPassive

  ; What is installed decides the words: install, update, same version or older.
  ; $CuradoState: "" nothing installed, 1 this one is newer, 0 same, -1 this one is older.
  StrCpy $CuradoState ""
  ReadRegStr $CuradoInstalled SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${If} $CuradoInstalled != ""
    nsis_tauri_utils::SemverCompare "${VERSION}" $CuradoInstalled
    Pop $CuradoState
  ${EndIf}
  ${If} $CuradoState == ""
    StrCpy $1 "$(curadoHello)"
    StrCpy $2 "$(curadoWhatFresh)"
    StrCpy $3 "$(curadoInstall)"
  ${ElseIf} $CuradoState = 1
    StrCpy $1 "$(curadoHelloUpdate)"
    StrCpy $2 "$(curadoWhatUpdate)"
    StrCpy $3 "$(curadoUpdate)"
  ${ElseIf} $CuradoState = 0
    StrCpy $1 "$(curadoHelloSame)"
    StrCpy $2 "$(curadoWhatSame)"
    StrCpy $3 "$(curadoNext)"
  ${Else}
    StrCpy $1 "$(curadoHelloSame)"
    StrCpy $2 "$(curadoWhatOlder)"
    StrCpy $3 "$(curadoNext)"
  ${EndIf}

  !insertmacro CURADO_FULL_PAGE
  !insertmacro CURADO_TEXT 124 10 195 34 "${PRODUCTNAME}" $NuFontMark "${NU_ACCENT_TEXT}"
  !insertmacro CURADO_TEXT 124 46 195 16 "$1" $NuFontDisplay "${NU_TEXT}"
  !insertmacro CURADO_TEXT 124 63 195 10 "$(curadoTagline)" "" "${NU_TEXT_2}"

  !insertmacro CURADO_FACT 82 18 "$(curadoWhat)" "$2"
  !insertmacro CURADO_TEXT 124 104 46 10 "$(curadoWhere)" $NuFontLabel "${NU_ACCENT_TEXT}"
  ${NSD_CreateLabel} 172u 104u 147u 10u "$INSTDIR"
  Pop $CuradoFolder
  ; SS_PATHELLIPSIS | SS_NOPREFIX: a long path keeps both ends, and "&" is a character
  ${NSD_AddStyle} $CuradoFolder 0x8080
  !insertmacro NU_COLOR $CuradoFolder "${NU_TEXT}" "${NU_SHEET}"
  ; The folder can change only on a first install: an update stays where the app is.
  ${If} $CuradoState == ""
    ${NSD_CreateButton} 172u 116u 54u 13u "$(curadoChange)"
    Pop $0
    ${NSD_OnClick} $0 CuradoPickFolder
    !insertmacro NU_DARK_BUTTON $0
  ${EndIf}
  !insertmacro CURADO_FACT 136 18 "$(curadoMusic)" "$(curadoMusicText)"
  !insertmacro CURADO_FACT 156 18 "$(curadoAdmin)" "$(curadoAdminText)"
  ${If} $NuDemo = 1
    !insertmacro CURADO_TEXT 124 179 195 10 "$(curadoDemo)" "" "${NU_DANGER}"
  ${Else}
    !insertmacro CURADO_TEXT 124 179 195 10 "$(curadoCredits)" "" "${NU_MUTED}"
  ${EndIf}

  SendMessage $mui.Button.Next ${WM_SETTEXT} 0 "STR:$3"
  SendMessage $mui.Button.Cancel ${WM_SETTEXT} 0 "STR:$(curadoNotNow)"
  ShowWindow $mui.Button.Back ${SW_HIDE}

  !insertmacro NU_FULL_PAGE_BEGIN
  nsDialogs::Show
  !insertmacro NU_FULL_PAGE_END
  ${NSD_FreeImage} $CuradoImageHandle
FunctionEnd

Function CuradoPickFolder
  Pop $0
  nsDialogs::SelectFolderDialog "$(curadoPickFolder)" "$INSTDIR"
  Pop $0
  ${If} $0 == error
  ${OrIf} $0 == ""
    Return
  ${EndIf}
  ; Keep the app in a folder of its own, as the directory page would.
  StrCpy $1 $0 1 -1
  ${IfThen} $1 == "\" ${|} StrCpy $0 $0 -1 ${|}
  ${GetFileName} "$0" $1
  ${IfThen} $1 != "${PRODUCTNAME}" ${|} StrCpy $0 "$0\${PRODUCTNAME}" ${|}
  StrCpy $INSTDIR $0
  ${NSD_SetText} $CuradoFolder "$INSTDIR"
FunctionEnd

Function CuradoWelcomeLeave
  ${If} $NuDemo = 1
    Return
  ${EndIf}
  ; Before anything is installed: the folder takes files and the app is closed.
  ClearErrors
  CreateDirectory "$INSTDIR"
  FileOpen $0 "$INSTDIR\.${PRODUCTNAME}-write-test" w
  ${If} ${Errors}
    MessageBox MB_OK|MB_ICONEXCLAMATION "$(curadoCannotWrite)"
    Abort
  ${EndIf}
  FileClose $0
  Delete "$INSTDIR\.${PRODUCTNAME}-write-test"
  !insertmacro NU_REQUIRE_APP_CLOSED "$INSTDIR\${MAINBINARYNAME}.exe" "$(curadoRunning)"
FunctionEnd

Function CuradoFinish
  Call SkipIfPassive

  !insertmacro CURADO_FULL_PAGE
  !insertmacro CURADO_TEXT 124 10 195 34 "$(curadoDone)" $NuFontMark "${NU_ACCENT_TEXT}"
  !insertmacro CURADO_TEXT 124 46 195 16 "$(curadoDoneText)" $NuFontDisplay "${NU_TEXT}"
  !insertmacro CURADO_TEXT 124 72 195 10 "$(curadoFirst)" $NuFontLabel "${NU_ACCENT_TEXT}"
  !insertmacro CURADO_TEXT 124 83 195 20 "$(curadoFirstText)" "" "${NU_TEXT}"

  ${NSD_CreateCheckbox} 124u 116u 195u 12u "$(curadoRunNow)"
  Pop $CuradoRun
  ${NSD_Check} $CuradoRun
  !insertmacro NU_CLASSIC $CuradoRun
  !insertmacro NU_COLOR $CuradoRun "${NU_TEXT}" "${NU_SHEET}"
  ${NSD_CreateCheckbox} 124u 131u 195u 12u "$(curadoDesktop)"
  Pop $CuradoDesktop
  ${NSD_Check} $CuradoDesktop
  !insertmacro NU_CLASSIC $CuradoDesktop
  !insertmacro NU_COLOR $CuradoDesktop "${NU_TEXT}" "${NU_SHEET}"
  ${If} $NoShortcutMode = 1
    ${NSD_Uncheck} $CuradoDesktop
    ShowWindow $CuradoDesktop ${SW_HIDE}
  ${EndIf}
  ${If} $NuDemo = 1
    EnableWindow $CuradoRun 0
    EnableWindow $CuradoDesktop 0
    !insertmacro CURADO_TEXT 124 152 195 10 "$(curadoDemoDone)" "" "${NU_DANGER}"
  ${EndIf}

  !if "${HOMEPAGE}" != ""
    ${NSD_CreateLink} 124u 179u 195u 10u "${HOMEPAGE}"
    Pop $0
    ${NSD_OnClick} $0 CuradoOpenSite
    !insertmacro NU_COLOR $0 "${NU_MUTED}" "${NU_SHEET}"
  !endif

  SendMessage $mui.Button.Next ${WM_SETTEXT} 0 "STR:$(curadoPlay)"
  ShowWindow $mui.Button.Cancel ${SW_HIDE}
  ShowWindow $mui.Button.Back ${SW_HIDE}

  !insertmacro NU_FULL_PAGE_BEGIN
  nsDialogs::Show
  !insertmacro NU_FULL_PAGE_END
  ${NSD_FreeImage} $CuradoImageHandle
FunctionEnd

Function CuradoFinishLeave
  ${If} $NuDemo = 1
    Return
  ${EndIf}
  ${NSD_GetState} $CuradoDesktop $0
  ${If} $0 = ${BST_CHECKED}
    Call CreateOrUpdateDesktopShortcut
  ${EndIf}
  ${NSD_GetState} $CuradoRun $0
  ${If} $0 = ${BST_CHECKED}
    Call RunMainBinary
  ${EndIf}
FunctionEnd

!if "${HOMEPAGE}" != ""
Function CuradoOpenSite
  Pop $0
  ExecShell "open" "${HOMEPAGE}"
FunctionEnd
!endif

Function .onInit
  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/NS" $NoShortcutMode
  ${IfNot} ${Errors}
    StrCpy $NoShortcutMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}

  ; curado: /DEMO and high contrast, read once for every page
  !insertmacro NU_READ_MODES
  !insertmacro NU_DETECT_HIGH_CONTRAST
  ${IfThen} $NuDemo = 1 ${|} StrCpy $CuradoDemoTag " · DEMO" ${|}

  !if "${DISPLAYLANGUAGESELECTOR}" == "true"
    !insertmacro MUI_LANGDLL_DISPLAY
  !endif

  !insertmacro SetContext

  ${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"
    ; Set default install location
    !if "${INSTALLMODE}" == "perMachine"
      ${If} ${RunningX64}
        !if "${ARCH}" == "x64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else if "${ARCH}" == "arm64"
          StrCpy $INSTDIR "$PROGRAMFILES64\${PRODUCTNAME}"
        !else
          StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
        !endif
      ${Else}
        StrCpy $INSTDIR "$PROGRAMFILES\${PRODUCTNAME}"
      ${EndIf}
    !else if "${INSTALLMODE}" == "currentUser"
      StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
    !endif

    Call RestorePreviousInstallLocation
  ${EndIf}


  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_INIT
  !endif

  ; Abort downgrades if they are disabled, in the modes that skip the reinstall page choice:
  ; silent, passive and update
  !if "${ALLOWDOWNGRADES}" == "false"
  ${If} ${Silent}
  ${OrIf} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    ; Compare with the same installation the reinstall page would: a WiX one first
    Call FindWixInstall
    ${If} $R6 != ""
      ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
    ${Else}
      ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
    ${EndIf}
    nsis_tauri_utils::SemverCompare "${VERSION}" $R0
    Pop $R0
    ; If downgrading
    ${If} $R0 = -1
      ${If} ${Silent}
        System::Call 'kernel32::AttachConsole(i -1)i.r0'
        ${If} $0 <> 0
          System::Call 'kernel32::GetStdHandle(i -11)i.r0'
          System::call 'kernel32::SetConsoleTextAttribute(i r0, i 0x0004)' ; set red color
          FileWrite $0 "$(silentDowngrades)"
        ${EndIf}
      ${Else}
        MessageBox MB_ICONSTOP "$(silentDowngrades)"
      ${EndIf}
      Abort
    ${EndIf}
  ${EndIf}
  !endif
FunctionEnd

Section WebView2
  ; curado: /DEMO writes nothing
  ${If} $NuDemo = 1
    Return
  ${EndIf}

  ; Check if Webview2 is already installed and skip this section
  ${If} ${RunningX64}
    ReadRegStr $4 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${Else}
    ReadRegStr $4 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}
  ${If} $4 == ""
    ReadRegStr $4 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
  ${EndIf}

  ${If} $4 == ""
    ; Webview2 installation
    ;
    ; Skip if updating
    ${If} $UpdateMode <> 1
      !if "${INSTALLWEBVIEW2MODE}" == "downloadBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        DetailPrint "$(webview2Downloading)"
        NSISdl::download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Pop $0
        ${If} $0 == "success"
          DetailPrint "$(webview2DownloadSuccess)"
        ${Else}
          DetailPrint "$(webview2DownloadError)"
          Abort "$(webview2AbortError)"
        ${EndIf}
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "embedBootstrapper"
        Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebview2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebview2Setup.exe"
        Goto install_webview2
      !endif

      !if "${INSTALLWEBVIEW2MODE}" == "offlineInstaller"
        Delete "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        File "/oname=$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe" "${WEBVIEW2INSTALLERPATH}"
        DetailPrint "$(installingWebview2)"
        StrCpy $6 "$TEMP\MicrosoftEdgeWebView2RuntimeInstaller.exe"
        Goto install_webview2
      !endif

      Goto webview2_done

      install_webview2:
        DetailPrint "$(installingWebview2)"
        ; $6 holds the path to the webview2 installer
        ExecWait "$6 ${WEBVIEW2INSTALLERARGS} /install" $1
        ${If} $1 = 0
          DetailPrint "$(webview2InstallSuccess)"
        ${Else}
          DetailPrint "$(webview2InstallError)"
          Abort "$(webview2AbortError)"
        ${EndIf}
      webview2_done:
    ${EndIf}
  ${Else}
    !if "${MINIMUMWEBVIEW2VERSION}" != ""
      ${VersionCompare} "${MINIMUMWEBVIEW2VERSION}" "$4" $R0
      ${If} $R0 = 1
        update_webview:
          DetailPrint "$(installingWebview2)"
          ${If} ${RunningX64}
            ReadRegStr $R1 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate" "path"
          ${Else}
            ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 == ""
            ReadRegStr $R1 HKCU "SOFTWARE\Microsoft\EdgeUpdate" "path"
          ${EndIf}
          ${If} $R1 != ""
            ; Chromium updater docs: https://source.chromium.org/chromium/chromium/src/+/main:docs/updater/user_manual.md
            ; Modified from "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Microsoft EdgeWebView\ModifyPath"
            ExecWait `"$R1" /install appguid=${WEBVIEW2APPGUID}&needsadmin=true` $1
            ${If} $1 = 0
              DetailPrint "$(webview2InstallSuccess)"
            ${Else}
              MessageBox MB_ICONEXCLAMATION|MB_ABORTRETRYIGNORE "$(webview2InstallError)" IDIGNORE ignore IDRETRY update_webview
              Quit
              ignore:
            ${EndIf}
          ${EndIf}
      ${EndIf}
    !endif
  ${EndIf}
SectionEnd

Section Install
  ; curado: /DEMO moves the progress bar and writes nothing
  ${If} $NuDemo = 1
    !insertmacro NU_DEMO_PROGRESS "$(curadoDemoStep)"
    Return
  ${EndIf}

  SetOutPath $INSTDIR

  !ifmacrodef NSIS_HOOK_PREINSTALL
    !insertmacro NSIS_HOOK_PREINSTALL
  !endif

  ; curado: never ends a running app (upstream force-closes it, silently in silent mode)
  !insertmacro NU_REQUIRE_APP_CLOSED "$INSTDIR\${MAINBINARYNAME}.exe" "$(curadoRunning)"

  ; Copy main executable
  File "${MAINBINARYSRCPATH}"

  ; Copy resources
  {{#each resources_dirs}}
    CreateDirectory "$INSTDIR\\{{this}}"
  {{/each}}
  {{#each resources}}
    File /a "/oname={{this.[1]}}" "{{no-escape @key}}"
  {{/each}}

  ; Copy external binaries
  {{#each binaries}}
    File /a "/oname={{this}}" "{{no-escape @key}}"
  {{/each}}

  ; Create file associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
       !insertmacro APP_ASSOCIATE "{{ext}}" "{{or association.name ext}}" "{{association-description association.description ext}}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0" "Open with ${PRODUCTNAME}" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
    {{/each}}
  {{/each}}

  ; Register deep links
  {{#each deep_link_protocols as |protocol| ~}}
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "URL Protocol" ""
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}" "" "URL:${BUNDLEID} protocol"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
    WriteRegStr SHCTX "Software\Classes\\{{protocol}}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  {{/each}}

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save $INSTDIR in registry for future installations
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR

  ; curado: 0.1.0 builds from before the publisher was set kept this under Software\curado
  !if "${MANUFACTURER}" != "${PRODUCTNAME}"
    DeleteRegKey SHCTX "Software\${PRODUCTNAME}\${PRODUCTNAME}"
    DeleteRegKey /ifempty SHCTX "Software\${PRODUCTNAME}"
  !endif

  !if "${INSTALLMODE}" == "both"
    ; Save install mode to be selected by default for the next installation such as updating
    ; or when uninstalling
    WriteRegStr SHCTX "${UNINSTKEY}" $MultiUser.InstallMode 1
  !endif

  ; Remove old main binary if it doesn't match new main binary name
  ReadRegStr $OldMainBinaryName SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    Delete "$INSTDIR\$OldMainBinaryName"
  ${EndIf}

  ; Save current MAINBINARYNAME for future updates
  WriteRegStr SHCTX "${UNINSTKEY}" "MainBinaryName" "${MAINBINARYNAME}.exe"

  ; Registry information for add/remove programs
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTKEY}" "Publisher" "${MANUFACTURER}"
  WriteRegStr SHCTX "${UNINSTKEY}" "InstallLocation" "$\"$INSTDIR$\""
  WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoModify" "1"
  WriteRegDWORD SHCTX "${UNINSTKEY}" "NoRepair" "1"

  ${GetSize} "$INSTDIR" "/M=uninstall.exe /S=0K /G=0" $0 $1 $2
  IntOp $0 $0 + ${ESTIMATEDSIZE}
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD SHCTX "${UNINSTKEY}" "EstimatedSize" "$0"

  !if "${HOMEPAGE}" != ""
    WriteRegStr SHCTX "${UNINSTKEY}" "URLInfoAbout" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "URLUpdateInfo" "${HOMEPAGE}"
    WriteRegStr SHCTX "${UNINSTKEY}" "HelpLink" "${HOMEPAGE}"
  !endif

  ; Create start menu shortcut
  !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
    Call CreateOrUpdateStartMenuShortcut
  !insertmacro MUI_STARTMENU_WRITE_END

  ; Create desktop shortcut for silent and passive installers
  ; because finish page will be skipped
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    Call CreateOrUpdateDesktopShortcut
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTINSTALL
    !insertmacro NSIS_HOOK_POSTINSTALL
  !endif

  ; Auto close this page for passive mode
  ${If} $PassiveMode = 1
    SetAutoClose true
  ${EndIf}
SectionEnd

Function .onInstSuccess
  ; curado: /DEMO starts nothing
  ${If} $NuDemo = 1
    Return
  ${EndIf}

  ; Check for `/R` flag only in silent and passive installers because
  ; GUI installer has a toggle for the user to (re)start the app
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    ${GetOptions} $CMDLINE "/R" $R0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $R0
      nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "$R0"
    ${EndIf}
  ${EndIf}
FunctionEnd

Function un.onInit
  !insertmacro SetContext
  !insertmacro NU_DETECT_HIGH_CONTRAST ; curado

  !if "${INSTALLMODE}" == "both"
    !insertmacro MULTIUSER_UNINIT
  !endif

  !insertmacro MUI_UNGETLANGUAGE

  ${GetOptions} $CMDLINE "/P" $PassiveMode
  ${IfNot} ${Errors}
    StrCpy $PassiveMode 1
  ${EndIf}

  ${GetOptions} $CMDLINE "/UPDATE" $UpdateMode
  ${IfNot} ${Errors}
    StrCpy $UpdateMode 1
  ${EndIf}
FunctionEnd

Section Uninstall

  !ifmacrodef NSIS_HOOK_PREUNINSTALL
    !insertmacro NSIS_HOOK_PREUNINSTALL
  !endif

  ; curado: never ends a running app (upstream force-closes it, silently in silent mode)
  !insertmacro NU_REQUIRE_APP_CLOSED "$INSTDIR\${MAINBINARYNAME}.exe" "$(curadoRunning)"

  ; Delete the app directory and its content from disk
  ; Copy main executable
  Delete "$INSTDIR\${MAINBINARYNAME}.exe"

  ; Delete resources
  {{#each resources}}
    Delete "$INSTDIR\\{{this.[1]}}"
  {{/each}}

  ; Delete external binaries
  {{#each binaries}}
    Delete "$INSTDIR\\{{this}}"
  {{/each}}

  ; Delete app associations
  {{#each file_associations as |association| ~}}
    {{#each association.ext as |ext| ~}}
      !insertmacro APP_UNASSOCIATE "{{ext}}" "{{or association.name ext}}"
    {{/each}}
  {{/each}}

  ; Delete deep links
  {{#each deep_link_protocols as |protocol| ~}}
    ReadRegStr $R7 SHCTX "Software\Classes\\{{protocol}}\shell\open\command" ""
    ${If} $R7 == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
      DeleteRegKey SHCTX "Software\Classes\\{{protocol}}"
    ${EndIf}
  {{/each}}


  ; Delete uninstaller
  Delete "$INSTDIR\uninstall.exe"

  {{#each resources_ancestors}}
  RMDir /REBOOTOK "$INSTDIR\\{{this}}"
  {{/each}}
  RMDir "$INSTDIR"

  ; Remove shortcuts if not updating
  ${If} $UpdateMode <> 1
    !insertmacro DeleteAppUserModelId

    ; Remove start menu shortcut
    !insertmacro MUI_STARTMENU_GETFOLDER Application $AppStartMenuFolder
    !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
      RMDir "$SMPROGRAMS\$AppStartMenuFolder"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}

    ; Remove desktop shortcuts
    !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}

  ; Remove registry information for add/remove programs
  !if "${INSTALLMODE}" == "both"
    DeleteRegKey SHCTX "${UNINSTKEY}"
  !else if "${INSTALLMODE}" == "perMachine"
    DeleteRegKey HKLM "${UNINSTKEY}"
  !else
    DeleteRegKey HKCU "${UNINSTKEY}"
  !endif

  ; Removes the Autostart entry for ${PRODUCTNAME} from the HKCU Run key if it exists.
  ; This ensures the program does not launch automatically after uninstallation if it exists.
  ; If it doesn't exist, it does nothing.
  ; We do this when not updating (to preserve the registry value on updates)
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${EndIf}

  ; Delete app data if the checkbox is selected
  ; and if not updating
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; Clear the install location $INSTDIR from registry
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"

    ; Clear the install language from registry
    DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"
    DeleteRegKey /ifempty HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    SetShellVarContext current
    RmDir /r "$APPDATA\${BUNDLEID}"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

  !ifmacrodef NSIS_HOOK_POSTUNINSTALL
    !insertmacro NSIS_HOOK_POSTUNINSTALL
  !endif

  ; Auto close if passive mode or updating
  ${If} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    SetAutoClose true
  ${Else}
    SetAutoClose false ; curado: the uninstaller stays to say what happened
  ${EndIf}
SectionEnd

Function RestorePreviousInstallLocation
  ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
  ; curado: 0.1.0 builds from before the publisher was set kept it under Software\curado
  !if "${MANUFACTURER}" != "${PRODUCTNAME}"
    ${IfThen} $4 == "" ${|} ReadRegStr $4 SHCTX "Software\${PRODUCTNAME}\${PRODUCTNAME}" "" ${|}
  !endif
  ${IfThen} $4 != "" ${|} StrCpy $INSTDIR $4 ${|}
FunctionEnd

Function Skip
  Abort
FunctionEnd

Function SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd
Function un.SkipIfPassive
  ${IfThen} $PassiveMode = 1  ${|} Abort ${|}
FunctionEnd

Function CreateOrUpdateStartMenuShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  StrCpy $R0 0

  !insertmacro IsShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R0 1
  ${EndIf}

  ${If} $R0 = 1
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  !if "${STARTMENUFOLDER}" != ""
    CreateDirectory "$SMPROGRAMS\$AppStartMenuFolder"
    CreateShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !else
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !endif
FunctionEnd

Function CreateOrUpdateDesktopShortcut
  ; We used to use product name as MAINBINARYNAME
  ; migrate old shortcuts to target the new MAINBINARYNAME
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro SetShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Return
  ${EndIf}

  ; Skip creating shortcut if in update mode or no shortcut mode
  ; but always create if migrating from wix
  ${If} $WixMode = 0
    ${If} $UpdateMode = 1
    ${OrIf} $NoShortcutMode = 1
      Return
    ${EndIf}
  ${EndIf}

  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
FunctionEnd
