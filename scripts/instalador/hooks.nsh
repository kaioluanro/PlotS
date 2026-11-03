; Ganchos do instalador. Registram o PlotS como programa que abre PDF ("Abrir com"), sem tocar
; no valor padrao de ".pdf": o leitor que o usuario ja escolheu continua sendo o padrao.
; Tudo e gravado em SHCTX, que no modo por usuario e a area do proprio usuario.

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".pdf" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
  WriteRegStr SHCTX "Software\Classes\PlotS.PDF" "" "PDF"
  WriteRegStr SHCTX "Software\Classes\PlotS.PDF\shell\open" "FriendlyAppName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "Software\Classes\PlotS.PDF\shell\open\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
  WriteRegStr SHCTX "Software\Classes\.pdf\OpenWithProgids" "PlotS.PDF" ""
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue SHCTX "Software\Classes\.pdf\OpenWithProgids" "PlotS.PDF"
  DeleteRegKey SHCTX "Software\Classes\PlotS.PDF"
  DeleteRegKey SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe"
!macroend
