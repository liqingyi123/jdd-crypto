; Bust Windows desktop/start-menu icon cache after upgrade by pointing
; shortcuts at a dedicated .ico (same path as exe keeps showing the old icon).
;
; Also silently uninstall the previous product name 「多多解密」 before
; installing 「多多工具箱」 (uninstall registry key is based on PRODUCTNAME).

!define JDD_LEGACY_PRODUCTNAME "多多解密"

; $R0 = UninstallString (may include quotes)
; $R1 = install dir (no quotes), may be empty
!macro JddRunLegacyUninstaller
  ${If} $R0 != ""
    ; Stop running binary so files can be removed (same MAINBINARYNAME).
    ExecWait 'taskkill /F /IM "${MAINBINARYNAME}.exe" /T' $0

    ; Prefer silent uninstall; _?= keeps ExecWait until NSIS uninstaller finishes.
    StrCpy $R0 "$R0 /S"
    ${If} $R1 != ""
      StrCpy $R0 "$R0 _?=$R1"
    ${EndIf}
    ClearErrors
    ExecWait '$R0' $0

    ${If} $R1 != ""
      Delete "$R1\uninstall.exe"
      RMDir "$R1"
    ${EndIf}
  ${EndIf}
!macroend

!macro JddCleanupLegacyProduct hive
  DeleteRegKey ${hive} "Software\Microsoft\Windows\CurrentVersion\Uninstall\${JDD_LEGACY_PRODUCTNAME}"
  DeleteRegKey ${hive} "Software\${MANUFACTURER}\${JDD_LEGACY_PRODUCTNAME}"
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; --- Uninstall legacy 「多多解密」 (currentUser / perMachine leftovers) ---
  StrCpy $R0 ""
  StrCpy $R1 ""

  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${JDD_LEGACY_PRODUCTNAME}" "UninstallString"
  ReadRegStr $R1 HKCU "Software\${MANUFACTURER}\${JDD_LEGACY_PRODUCTNAME}" ""
  ${If} $R0 != ""
    !insertmacro JddRunLegacyUninstaller
    !insertmacro JddCleanupLegacyProduct HKCU
  ${EndIf}

  StrCpy $R0 ""
  StrCpy $R1 ""
  ReadRegStr $R0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${JDD_LEGACY_PRODUCTNAME}" "UninstallString"
  ReadRegStr $R1 HKLM "Software\${MANUFACTURER}\${JDD_LEGACY_PRODUCTNAME}" ""
  ${If} $R0 != ""
    !insertmacro JddRunLegacyUninstaller
    !insertmacro JddCleanupLegacyProduct HKLM
  ${EndIf}

  ; Leftover shortcuts from the old DisplayName
  Delete "$DESKTOP\${JDD_LEGACY_PRODUCTNAME}.lnk"
  Delete "$SMPROGRAMS\${JDD_LEGACY_PRODUCTNAME}.lnk"
  Delete "$SMPROGRAMS\${JDD_LEGACY_PRODUCTNAME}\*.*"
  RMDir "$SMPROGRAMS\${JDD_LEGACY_PRODUCTNAME}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\desktop-icon.ico" 0 jdd_icon_done

  IfFileExists "$DESKTOP\${PRODUCTNAME}.lnk" 0 jdd_icon_startmenu
    Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\desktop-icon.ico" 0
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"

  jdd_icon_startmenu:
  IfFileExists "$SMPROGRAMS\${PRODUCTNAME}.lnk" 0 jdd_icon_notify
    Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\desktop-icon.ico" 0
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"

  jdd_icon_notify:
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'

  jdd_icon_done:
!macroend
