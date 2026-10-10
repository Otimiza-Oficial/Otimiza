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

; ATUALIZAÇÃO com o Otimiza aberto COMO ADMINISTRADOR: o instalador (sem administrador) não consegue fechá-lo nem trocar o
; programa, e terminava sem erro com a versão velha no lugar (aconteceu com o dono em 09/10/2026: "instalei e não
; aparece"). Aqui ele confere de verdade se o processo ainda existe e não segue até ele fechar.
!macro NSIS_HOOK_PREINSTALL
  StrCpy $R7 0
  otimiza_confere_aberto:
    nsExec::ExecToStack 'cmd /c tasklist /NH /FI "IMAGENAME eq ${MAINBINARYNAME}.exe" | find /I "${MAINBINARYNAME}.exe"'
    Pop $R6
    Pop $R5
    ${If} $R6 == "0"
      IfSilent 0 otimiza_pergunta
        ; Silencioso: espera até 60 s e desiste, para não instalar pela metade.
        IntOp $R7 $R7 + 1
        ${If} $R7 > 30
          Abort
        ${EndIf}
        Sleep 2000
        Goto otimiza_confere_aberto
      otimiza_pergunta:
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "O Otimiza ainda está aberto (talvez na bandeja, perto do relógio, ou como administrador) e o Windows não deixa trocar o programa com ele rodando.$\r$\n$\r$\nFeche o Otimiza: clique com o botão direito no ícone da bandeja e saia, ou finalize $\"${MAINBINARYNAME}.exe$\" no Gerenciador de Tarefas (Ctrl+Shift+Esc). Depois clique em Repetir." IDRETRY otimiza_confere_aberto
      Abort
    ${EndIf}
!macroend
