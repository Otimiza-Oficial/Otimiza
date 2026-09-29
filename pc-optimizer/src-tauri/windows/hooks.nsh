; Ganchos do instalador do Otimiza (MASTER-PLAN, item 6): ao desinstalar, oferece desfazer o que o Otimiza mudou no
; PC. O próprio programa faz o trabalho (`pc-optimizer.exe --restaurar-tudo`, ver src/modules/windows/restaurar.rs) e
; mostra o resultado; aqui só se pergunta, se chama e se decide se a desinstalação continua.
;
; - Na ATUALIZAÇÃO o instalador novo roda este desinstalador com /UPDATE: nada é desfeito, as mudanças seguem valendo.
; - Silencioso (/S) ou passivo (/P): não pergunta e não desfaz; quem desinstala por script decide por si.
; - Este gancho roda ANTES de o desinstalador pedir para fechar o Otimiza: por isso pede aqui mesmo, antes de chamar.
; - O desinstalador não é administrador (instalação por usuário); o "runas" pede a permissão só para restaurar.
; - `ExecShellWait` não devolve o código de saída: o programa o grava num arquivo na pasta de dados. Sem o arquivo
;   (permissão negada) ou com qualquer coisa além de 0, pergunta se desinstala mesmo assim; "Não" cancela.

!define OTIMIZA_RESULTADO "$APPDATA\pc-optimizer\restauracao-resultado.txt"

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} $PassiveMode <> 1
    IfSilent otimiza_fim_da_restauracao
    MessageBox MB_YESNO|MB_ICONQUESTION "Desfazer o que o Otimiza mudou neste PC antes de desinstalar?$\r$\n$\r$\nO Otimiza será fechado e o Windows vai pedir permissão de administrador." IDNO otimiza_fim_da_restauracao
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
    Delete "${OTIMIZA_RESULTADO}"
    ExecShellWait "runas" "$INSTDIR\${MAINBINARYNAME}.exe" '--restaurar-tudo --dados "$APPDATA"'
    StrCpy $R9 ""
    ClearErrors
    FileOpen $R8 "${OTIMIZA_RESULTADO}" r
    ${IfNot} ${Errors}
      FileRead $R8 $R9
      FileClose $R8
    ${EndIf}
    Delete "${OTIMIZA_RESULTADO}"
    ${If} $R9 != "0"
      MessageBox MB_YESNO|MB_ICONEXCLAMATION "O Otimiza não desfez tudo: a permissão foi negada, ou a mensagem anterior diz o que ficou.$\r$\n$\r$\nDesinstalar mesmo assim? Escolha Não para manter o Otimiza e desfazer pelo botão $\"Desfazer tudo$\"." IDYES otimiza_fim_da_restauracao
      Abort
    ${EndIf}
    otimiza_fim_da_restauracao:
  ${EndIf}
!macroend
