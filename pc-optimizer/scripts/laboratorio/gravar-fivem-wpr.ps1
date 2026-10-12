# Laboratório E6: 30 s do FiveM gravados pelo Windows Performance Recorder (o gravador de desempenho do próprio
# Windows), para achar onde o processador trava. Grava uso do processador (com pilhas), trocas de thread, interrupções
# de driver (DPC/ISR), frequência e núcleos estacionados. Não muda nada no PC. Precisa de administrador.
param([int]$Segundos = 30)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Write-Host 'Abra o PowerShell como administrador e rode de novo.'; exit 1 }

$pasta = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Otimiza-laboratorio\wpr'
New-Item -ItemType Directory -Force $pasta | Out-Null
$arquivo = Join-Path $pasta ("fivem-{0:yyyy-MM-dd_HH-mm}.etl" -f (Get-Date))

if (-not (Get-Process | Where-Object { $_.ProcessName -like 'FiveM*GTAProcess*' })) {
  Write-Host 'O FiveM nao esta aberto. Abra, entre na cidade e rode de novo.'; exit 1
}
& wpr.exe -cancel 2>$null | Out-Null
Write-Host ''
Write-Host 'Volte para o FiveM e jogue normalmente (dirigindo pela cidade, de preferencia).'
Write-Host "A gravacao comeca em 10 s e dura $Segundos s."
Start-Sleep -Seconds 10
& wpr.exe -start CPU -start Power -filemode
if ($LASTEXITCODE -ne 0) { Write-Host 'O WPR nao conseguiu comecar.'; exit 1 }
[console]::beep(880, 200)
Start-Sleep -Seconds $Segundos
& wpr.exe -stop $arquivo 'Otimiza laboratorio E6: FiveM' | Out-Null
[console]::beep(660, 300)
if (Test-Path $arquivo) {
  "Pronto: {0} ({1:N0} MB)" -f $arquivo, ((Get-Item $arquivo).Length / 1MB)
} else {
  'A gravacao nao foi salva.'
}
