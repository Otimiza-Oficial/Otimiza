@echo off
rem ===========================================================================
rem  Gera o par de chaves do Otimiza. Clique duas vezes neste arquivo.
rem
rem  A CHAVE PRIVADA NAO APARECE NA TELA. Ela vai do gerador direto para o
rem  .env do bot.
rem
rem  POR QUE ASSIM
rem
rem  A primeira versao imprimia as duas metades e mandava copiar cada uma para
rem  o seu lugar. A privada foi parar em `.env.example` — o arquivo VERSIONADO,
rem  nao o `.env` — duas vezes em quinze minutos, e nas duas apareceu numa
rem  captura de tela.
rem
rem  Duas vezes seguidas nao e desatencao: e um projeto que pede a coisa
rem  errada. Um segredo que precisa passar pela tela e pela area de
rem  transferencia ate um arquivo de nome quase identico ao errado vai parar no
rem  arquivo errado. Entao ele deixou de passar pela tela.
rem ===========================================================================

chcp 65001 >nul
cd /d "%~dp0"

echo.
echo   OTIMIZA - GERADOR DE PAR DE CHAVES
echo   ==================================
echo.
echo   Isto roda UMA vez na vida do produto.
echo.
echo   A chave PRIVADA vai direto para o .env do bot e nao aparece aqui.
echo   So a PUBLICA e mostrada, e ela pode ser vista por qualquer um.
echo.

where node >nul 2>nul
if errorlevel 1 (
    echo   O Node nao foi encontrado neste computador.
    echo   Instale em https://nodejs.org e rode este arquivo de novo.
    echo.
    pause
    exit /b 1
)

echo   Arraste a pasta do bot para esta janela e tecle Enter.
echo   (ou digite o caminho, por exemplo: C:\Users\Voce\Downloads\T\qrbot)
echo.
set /p PASTA="   Pasta do bot: "

rem Aspas atrapalham quando a pasta e arrastada; saem aqui.
set PASTA=%PASTA:"=%

if not exist "%PASTA%" (
    echo.
    echo   Nao achei a pasta "%PASTA%".
    echo.
    pause
    exit /b 1
)

echo.
node "%~dp0otimiza-licenca.cjs" instalar "%PASTA%\.env"

if errorlevel 1 (
    echo.
    echo   Nada foi gerado.
    echo.
    pause
    exit /b 1
)

echo   ---------------------------------------------------------------
echo   A PRIVADA ja esta no .env do bot. Nao procure por ela, nao copie,
echo   e nao mande para ninguem — nem para mim.
echo.
echo   Guarde uma copia do .env num lugar seguro: perder a privada
echo   obriga a reemitir a licenca de todos os clientes.
echo   ---------------------------------------------------------------
echo.
pause
