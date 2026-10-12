# Mede o FiveM com o PresentMon: FPS médio, 1% low e os quadros lentos, e guarda com um nome (o estado do PC).
# Precisa de administrador (o PresentMon lê o ETW). Jogue o mesmo trajeto em todas as medidas.
#   powershell -ExecutionPolicy Bypass -File medir-fivem.ps1 -Rotulo pacote-ligado
param(
  [Parameter(Mandatory = $true)][string]$Rotulo,
  [int]$Segundos = 60,
  [string]$Jogo = 'FiveM_b3258_GTAProcess.exe'
)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Write-Host 'Abra o PowerShell como administrador e rode de novo.'; exit 1 }
if (-not (Get-Process | Where-Object { "$($_.ProcessName).exe" -eq $Jogo })) { Write-Host "O $Jogo nao esta aberto."; exit 1 }

$pm = Join-Path $env:LOCALAPPDATA 'Otimiza\PresentMon.exe'
$pasta = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Otimiza-laboratorio\medidas'
New-Item -ItemType Directory -Force $pasta | Out-Null
$csv = Join-Path $pasta ("{0}-{1:yyyy-MM-dd_HH-mm}.csv" -f $Rotulo, (Get-Date))

Write-Host 'Volte para o FiveM e faca o trajeto de sempre. A medida comeca em 10 s (bipe) e dura' $Segundos 's.'
Start-Sleep -Seconds 10
[console]::beep(880, 200)
& $pm --process_name $Jogo --timed $Segundos --terminate_after_timed --output_file $csv --no_console_stats `
  --session_name OtimizaMedida --stop_existing_session *> $null
[console]::beep(660, 300)
if (-not (Test-Path $csv)) { Write-Host 'O PresentMon nao gravou nada.'; exit 1 }

$linhas = Import-Csv $csv
$coluna = if ($linhas[0].PSObject.Properties.Name -contains 'MsBetweenPresents') { 'MsBetweenPresents' } else { 'FrameTime' }
$tempos = @($linhas | ForEach-Object { [double]::Parse($_.$coluna, [Globalization.CultureInfo]::InvariantCulture) } | Where-Object { $_ -gt 0 })
$ordenados = $tempos | Sort-Object
$total = ($tempos | Measure-Object -Sum).Sum
$medio = 1000 * $tempos.Count / $total
$pior1 = $ordenados[[int][Math]::Floor($ordenados.Count * 0.99)]
$lentos = @($tempos | Where-Object { $_ -gt 40 }).Count
$resumo = "{0} | {1:N1} fps medio | 1% low {2:N1} fps | quadro p99 {3:N1} ms | acima de 40 ms: {4} | {5} quadros em {6:N0} s" -f `
  $Rotulo, $medio, (1000 / $pior1), $pior1, $lentos, $tempos.Count, ($total / 1000)
$resumo
Add-Content -Path (Join-Path $pasta 'resumo.txt') -Value ("{0:yyyy-MM-dd HH:mm} | {1}" -f (Get-Date), $resumo)
