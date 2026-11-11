; nu9ve installer kit for NSIS 3 with Modern UI 2.
;
; App-agnostic: no product name, text or page lives here. It gives an installer the
; app's theme (colors from palette.nsh, a display font, dark window chrome) and three
; behaviours the suite asks of every installer:
;   - high contrast: Windows' own colors win over the theme;
;   - an open app is never ended, the user closes it;
;   - /DEMO walks every page and writes nothing.
;
; Tauri includes this file in its `installerHooks` slot, before the template's own
; defines exist, so it holds only defines, variables and macros. The template inserts
; the macros once PRODUCTNAME and the languages are known. Keep this file ASCII: makensis
; reads an included file without a byte-order mark in the system code page.

!ifndef NU_KIT_DIR
!define NU_KIT_DIR "${__FILEDIR__}"
!include "${NU_KIT_DIR}\palette.nsh"

; Header and standard pages take the theme from Modern UI itself.
!define MUI_BGCOLOR "${NU_CHROME}"
!define MUI_TEXTCOLOR "${NU_TEXT}"

!define /ifndef PBM_SETBARCOLOR 0x0409
!define /ifndef PBM_SETBKCOLOR 0x2001
!define /ifndef PBM_SETRANGE32 0x0406
!define /ifndef PBM_SETPOS 0x0402
!define /ifndef LVM_SETBKCOLOR 0x1001
!define /ifndef LVM_SETTEXTCOLOR 0x1024
!define /ifndef LVM_SETTEXTBKCOLOR 0x1026

Var NuHighContrast
Var NuDemo
Var NuFontMark
Var NuFontDisplay
Var NuFontLabel

; ---- Modes ------------------------------------------------------------------

; Reads the command line. /DEMO shows every page and writes nothing to the system.
!macro NU_READ_MODES
  StrCpy $NuDemo 0
  ClearErrors
  ${GetOptions} $CMDLINE "/DEMO" $R0
  ${IfNot} ${Errors}
    StrCpy $NuDemo 1
  ${EndIf}
  ClearErrors
!macroend

; SPI_GETHIGHCONTRAST. The installer is a 32-bit process, so HIGHCONTRAST is 12 bytes.
!macro NU_DETECT_HIGH_CONTRAST
  StrCpy $NuHighContrast 0
  System::Call '*(i 12, i 0, p 0) p .r0'
  ${If} $0 P<> 0
    System::Call 'user32::SystemParametersInfoW(i 0x42, i 12, p r0, i 0) i .r1'
    ${If} $1 <> 0
      System::Call '*$0(i, i .r1, p)'
      IntOp $NuHighContrast $1 & 1
    ${EndIf}
    System::Free $0
  ${EndIf}
!macroend

; ---- Colors and themes ------------------------------------------------------

; Colors for one control or dialog. Text and background are always set together, so a
; half-applied theme cannot leave unreadable text; in high contrast both are the system's.
!macro NU_COLOR HWND FG BG
  ${If} $NuHighContrast = 1
    SetCtlColors ${HWND} SYSCLR:8 SYSCLR:5
  ${Else}
    SetCtlColors ${HWND} "${FG}" "${BG}"
  ${EndIf}
!macroend

; A themed check box or radio button ignores its text color. Classic drawing honors it.
!macro NU_CLASSIC HWND
  ${If} $NuHighContrast <> 1
    System::Call 'uxtheme::SetWindowTheme(p s, w " ", w " ")' ${HWND}
  ${EndIf}
!macroend

; Windows' own dark push button. Older builds keep the light one, which still reads.
!macro NU_DARK_BUTTON HWND
  ${If} $NuHighContrast <> 1
    System::Call 'uxtheme::SetWindowTheme(p s, w "DarkMode_Explorer", p 0)' ${HWND}
  ${EndIf}
!macroend

; The display font ships inside the installer and is registered for this process only.
!macro NU_LOAD_FONTS
  InitPluginsDir
  File "/oname=$PLUGINSDIR\${NU_DISPLAY_FONT_FILE}" "${NU_KIT_DIR}\${NU_DISPLAY_FONT_FILE}"
  System::Call 'gdi32::AddFontResourceExW(w "$PLUGINSDIR\${NU_DISPLAY_FONT_FILE}", i 0x10, p 0) i .r0'
  CreateFont $NuFontMark "${NU_DISPLAY_FONT}" 34 400
  CreateFont $NuFontDisplay "${NU_DISPLAY_FONT}" 16 400
  CreateFont $NuFontLabel "$(^Font)" 7 700
!macroend

; Outer dialog: dark caption with the accent as the window border (Windows 11; older
; builds ignore the attributes), flat chrome without etched lines or branding text, and
; the display font in the header.
!macro NU_OUTER_DIALOG
  ${If} $NuHighContrast = 1
    SetCtlColors $mui.Header.Text SYSCLR:8 SYSCLR:5
    SetCtlColors $mui.Header.SubText SYSCLR:8 SYSCLR:5
    SetCtlColors $mui.Header.Background SYSCLR:8 SYSCLR:5
  ${Else}
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, *i 1, i 4)'
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 35, *i ${NU_CHROME_REF}, i 4)'
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 36, *i ${NU_TEXT_REF}, i 4)'
    System::Call 'dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 34, *i ${NU_ACCENT_REF}, i 4)'
    SetCtlColors $mui.Header.SubText "${NU_TEXT_2}" "${NU_CHROME}"
    SendMessage $mui.Header.Text ${WM_SETFONT} $NuFontDisplay 0
  ${EndIf}
  !insertmacro NU_COLOR $HWNDPARENT "${NU_TEXT}" "${NU_CHROME}"
  ShowWindow $mui.Line.Standard ${SW_HIDE}
  ShowWindow $mui.Line.FullWindow ${SW_HIDE}
  ShowWindow $mui.Branding.Background ${SW_HIDE}
  ShowWindow $mui.Branding.Text ${SW_HIDE}
  !insertmacro NU_DARK_BUTTON $mui.Button.Next
  !insertmacro NU_DARK_BUTTON $mui.Button.Cancel
  !insertmacro NU_DARK_BUTTON $mui.Button.Back
!macroend

; A page that covers the header: hide what would show through it, and put it back after.
!macro NU_FULL_PAGE_BEGIN
  LockWindow on
  ShowWindow $mui.Header.Text ${SW_HIDE}
  ShowWindow $mui.Header.SubText ${SW_HIDE}
  ShowWindow $mui.Header.Image ${SW_HIDE}
  LockWindow off
!macroend
!macro NU_FULL_PAGE_END
  LockWindow on
  ShowWindow $mui.Header.Text ${SW_NORMAL}
  ShowWindow $mui.Header.SubText ${SW_NORMAL}
  ShowWindow $mui.Header.Image ${SW_NORMAL}
  LockWindow off
!macroend

; The sheet a standard page sits on, and its text. $R9 is the inner dialog afterwards.
!macro NU_INNER_PAGE
  FindWindow $R9 "#32770" "" $HWNDPARENT
  !insertmacro NU_COLOR $R9 "${NU_TEXT}" "${NU_SHEET}"
!macroend
!macro NU_INNER_TEXT ID FG
  GetDlgItem $R8 $R9 ${ID}
  !insertmacro NU_COLOR $R8 "${FG}" "${NU_SHEET}"
!macroend

; Progress page: a classic block progress bar in the accent and a dark details list.
; Call from the page's SHOW function, where Modern UI has filled its handles.
!macro NU_STYLE_PROGRESS_PAGE
  !insertmacro NU_COLOR $mui.InstFilesPage "${NU_TEXT}" "${NU_SHEET}"
  !insertmacro NU_COLOR $mui.InstFilesPage.Text "${NU_TEXT_2}" "${NU_SHEET}"
  !insertmacro NU_DARK_BUTTON $mui.InstFilesPage.ShowLogButton
  ${If} $NuHighContrast <> 1
    System::Call 'uxtheme::SetWindowTheme(p s, w " ", w " ")' $mui.InstFilesPage.ProgressBar
    SendMessage $mui.InstFilesPage.ProgressBar ${PBM_SETBKCOLOR} 0 ${NU_CHROME_REF}
    SendMessage $mui.InstFilesPage.ProgressBar ${PBM_SETBARCOLOR} 0 ${NU_ACCENT_REF}
    SendMessage $mui.InstFilesPage.Log ${LVM_SETBKCOLOR} 0 ${NU_CHROME_REF}
    SendMessage $mui.InstFilesPage.Log ${LVM_SETTEXTBKCOLOR} 0 ${NU_CHROME_REF}
    SendMessage $mui.InstFilesPage.Log ${LVM_SETTEXTCOLOR} 0 ${NU_TEXT_2_REF}
  ${EndIf}
!macroend

; ---- Behaviour --------------------------------------------------------------

; Stops while EXE is open and never ends it: the app may be playing. With a window the
; user closes the app and retries; silent and passive runs print MESSAGE and abort
; (exit code 2). In a page callback Abort keeps the page; in a section it ends the run.
!macro NU_REQUIRE_APP_CLOSED EXE MESSAGE
  !define NuCheck ${__COUNTER__}
  nu_check_${NuCheck}:
  StrCpy $R1 0
  !insertmacro RestartManager_StartSession $R0
  ${If} $R0 != ""
    !insertmacro RestartManager_RegisterFile $R0 "${EXE}"
    ${If} $0 = 0
      System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
      ; ERROR_MORE_DATA: at least one process holds the file
      ${If} $0 = 234
        StrCpy $R1 1
      ${EndIf}
    ${EndIf}
    !insertmacro RestartManager_EndSession $R0
  ${EndIf}
  ${If} $R1 = 1
    ${If} ${Silent}
      System::Call 'kernel32::AttachConsole(i -1) i .r0'
      ${If} $0 <> 0
        System::Call 'kernel32::GetStdHandle(i -11) p .r0'
        FileWrite $0 "${MESSAGE}$\n"
      ${EndIf}
      Abort "${MESSAGE}"
    ${EndIf}
    ${If} $PassiveMode = 1
      Abort "${MESSAGE}"
    ${EndIf}
    MessageBox MB_RETRYCANCEL|MB_ICONINFORMATION "${MESSAGE}" IDRETRY nu_check_${NuCheck}
    Abort "${MESSAGE}"
  ${EndIf}
  !undef NuCheck
!macroend

; /DEMO: the progress page moves like a real run while nothing is written.
; A silent run has no progress page, and must not go looking for one.
!macro NU_DEMO_PROGRESS TEXT
  ${IfNot} ${Silent}
  ${AndIf} $mui.InstFilesPage.ProgressBar P<> 0
    SendMessage $mui.InstFilesPage.ProgressBar ${PBM_SETRANGE32} 0 30
    ${ForEach} $R7 1 30 + 1
      SendMessage $mui.InstFilesPage.ProgressBar ${PBM_SETPOS} $R7 0
      DetailPrint "${TEXT}"
      Sleep 60
    ${Next}
  ${EndIf}
!macroend

!endif
