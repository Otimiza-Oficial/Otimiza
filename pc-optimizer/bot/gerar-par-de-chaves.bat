@echo off
rem ===========================================================================
rem  Gera o par de chaves do Otimiza. Clique duas vezes neste arquivo.
rem
rem  POR QUE ISTO EXISTE
rem
rem  O manual mandava digitar um caminho relativo num terminal, e caminho
rem  relativo so funciona se a pessoa estiver exatamente na pasta certa. Como
rem  existem duas pastas de nome parecido na maquina do dono, isso falhou duas
rem  vezes seguidas.
rem
rem  O `cd /d "%~dp0"` abaixo resolve de vez: `%~dp0` e a pasta DESTE arquivo,
rem  entao o script sempre roda no lugar certo, seja de onde for chamado.
rem ===========================================================================

rem Pagina de codigo 65001 = UTF-8. Sem isto os acentos saem quebrados.
chcp 65001 >nul
cd /d "%~dp0"

echo.
echo   OTIMIZA - GERADOR DE PAR DE CHAVES
echo   ==================================
echo.
echo   Isto roda UMA vez na vida do produto.
echo.
echo   Nao rode com alguem olhando a tela, nem com gravacao ligada:
echo   a chave PRIVADA vai aparecer aqui, e ela nao pode ser vista
echo   por mais ninguem.
echo.
pause

where node >nul 2>nul
if errorlevel 1 (
    echo.
    echo   O Node nao foi encontrado neste computador.
    echo   Instale em https://nodejs.org e rode este arquivo de novo.
    echo.
    pause
    exit /b 1
)

echo.
node "%~dp0otimiza-licenca.cjs" novo-par
echo.
echo   ---------------------------------------------------------------
echo   A PUBLICA vai para CHAVE_PUBLICA em:
echo   src-tauri\src\modules\licenca.rs
echo.
echo   A PRIVADA vai para o .env do bot (OTIMIZA_CHAVE_PRIVADA) e para
echo   um segundo lugar seguro. Ela NUNCA entra no repositorio e nunca
echo   e colada em conversa nenhuma.
echo.
echo   Perder a privada obriga a reemitir a licenca de todos os clientes.
echo   ---------------------------------------------------------------
echo.
pause
