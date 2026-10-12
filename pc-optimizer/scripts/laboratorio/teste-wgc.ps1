# Laboratório E1: só a Windows.Graphics.Capture no jogo de teste, para ver se ela abre neste PC.
$raiz = Join-Path $PSScriptRoot '..\..\src-tauri\target\release\examples'
$marcador = Join-Path $env:TEMP 'otimiza-wgc-marcador.txt'
$saida = Join-Path $env:TEMP 'otimiza-wgc-resultado.txt'
$env:OTIMIZA_TESTE_X = '1280'; $env:OTIMIZA_TESTE_Y = '0'; $env:OTIMIZA_TESTE_COPIA = '1'; $env:OTIMIZA_TESTE_MARCADOR = $marcador
$p = Start-Process -FilePath (Join-Path $raiz 'jogo_de_teste.exe') -ArgumentList @('60', '640', '360', '0') -PassThru
Start-Sleep -Seconds 2
& (Join-Path $raiz 'bancada_captura.exe') wgc 5 $marcador *> $saida
Stop-Process -Id $p.Id -Force
Get-Content $saida
"(resultado também em $saida)"
