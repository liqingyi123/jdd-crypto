; Bust Windows desktop/start-menu icon cache after upgrade by pointing
; shortcuts at a dedicated .ico (same path as exe keeps showing the old icon).

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
