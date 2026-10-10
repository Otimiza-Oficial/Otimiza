# Placar do Frame Boost: o mesmo trecho do jogo medido em vários modos (tela cheia, sem bordas, com o Frame Boost...),
# com o PresentMon no JOGO e no OTIMIZA ao mesmo tempo (a sobreposição do gerador é do processo do Otimiza).
#
# Precisa de administrador (o PresentMon lê o ETW): rode elevado UMA vez. Entre as etapas ele espera o arquivo
# "<Pasta>\vai-<rotulo>.flag": quem conduz troca o modo do jogo e cria o arquivo. Depois:
#   node scripts/laboratorio/ab-resumo.mjs <Pasta>
param(
  [Parameter(Mandatory = $true)][string]$Pasta,
  [string[]]$Rotulos = @('1-tela-cheia', '2-sem-bordas', '3-sem-bordas-frame-boost'),
  [string]$Jogo = 'FiveM_b3258_GTAProcess.exe',
  [int]$Segundos = 60,
  [int]$Espera = 10
)
# Com `powershell -File`, "a,b,c" chega como UM rótulo (o script esperava um arquivo com vírgulas no nome).
$Rotulos = @($Rotulos | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$pm = Join-Path $env:LOCALAPPDATA 'Otimiza\PresentMon.exe'
New-Item -ItemType Directory -Force $Pasta | Out-Null
foreach ($r in $Rotulos) {
  $limite = (Get-Date).AddMinutes(30)
  while (-not (Test-Path (Join-Path $Pasta "vai-$r.flag")) -and (Get-Date) -lt $limite) { Start-Sleep -Seconds 1 }
  if (-not (Test-Path (Join-Path $Pasta "vai-$r.flag"))) { break }
  Start-Sleep -Seconds $Espera
  & $pm --process_name $Jogo --process_name 'pc-optimizer.exe' --timed $Segundos --terminate_after_timed `
    --output_file (Join-Path $Pasta "$r.csv") --v2_metrics --track_frame_type --no_console_stats `
    --session_name OtimizaPlacar --stop_existing_session *> (Join-Path $Pasta "$r.txt")
  "fim $LASTEXITCODE" | Out-File (Join-Path $Pasta "$r.fim") -Encoding utf8
}
