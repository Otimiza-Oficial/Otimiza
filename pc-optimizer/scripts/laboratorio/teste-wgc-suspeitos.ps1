# Laboratório E1: o que bloqueia a Windows.Graphics.Capture neste PC? Liga, UM POR VEZ e acumulando, o que costuma vir
# desligado por "otimizadores" (inclusive o ajuste "Desativar gravação em segundo plano" do próprio Otimiza), testa a
# captura em C++ depois de cada passo e, no fim, DEVOLVE TUDO como estava. Precisa de administrador.
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Write-Host 'Abra o PowerShell como administrador e rode de novo.'; exit 1 }

$teste = Join-Path $PSScriptRoot '..\..\laboratorio\wgc-cpp\teste_wgc.exe'
$gamedvr = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\GameDVR'
$config = 'HKCU:\System\GameConfigStore'
$politica = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\GameDVR'
$servicos = @('BcastDVRUserService') + (Get-ChildItem 'HKLM:\SYSTEM\CurrentControlSet\Services' | Where-Object { $_.PSChildName -like 'BcastDVRUserService_*' } | ForEach-Object { $_.PSChildName })

function Ler($k, $n) { (Get-ItemProperty $k -Name $n -ErrorAction SilentlyContinue).$n }
$antes = [ordered]@{
  AppCaptureEnabled = Ler $gamedvr 'AppCaptureEnabled'
  GameDVR_Enabled   = Ler $config 'GameDVR_Enabled'
  AllowGameDVR      = Ler $politica 'AllowGameDVR'
}
$inicioDosServicos = @{}
foreach ($s in $servicos) { $inicioDosServicos[$s] = Ler "HKLM:\SYSTEM\CurrentControlSet\Services\$s" 'Start' }
"Antes: " + (($antes.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ', ') + ', serviços: ' + (($inicioDosServicos.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ', ')

function Testar($passo) { "{0} -> {1}" -f $passo, ((& $teste monitor) -join ' | ') }

try {
  Testar '0. como está'
  Set-ItemProperty $gamedvr -Name AppCaptureEnabled -Value 1 -Type DWord
  Set-ItemProperty $config -Name GameDVR_Enabled -Value 1 -Type DWord
  Testar '1. gravação do Game Bar ligada (AppCaptureEnabled e GameDVR_Enabled = 1)'
  foreach ($s in $servicos) { Set-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$s" -Name Start -Value 3 -Type DWord }
  foreach ($s in $servicos) { if ($s -ne 'BcastDVRUserService') { Start-Service $s -ErrorAction SilentlyContinue } }
  Testar '2. + serviço de gravação BcastDVR em manual'
  Remove-ItemProperty $politica -Name AllowGameDVR -ErrorAction SilentlyContinue
  Testar '3. + sem a política AllowGameDVR'
} finally {
  foreach ($s in $servicos) {
    if ($s -ne 'BcastDVRUserService') { Stop-Service $s -Force -ErrorAction SilentlyContinue }
    if ($null -ne $inicioDosServicos[$s]) { Set-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$s" -Name Start -Value $inicioDosServicos[$s] -Type DWord }
  }
  if ($null -ne $antes.AppCaptureEnabled) { Set-ItemProperty $gamedvr -Name AppCaptureEnabled -Value $antes.AppCaptureEnabled -Type DWord } else { Remove-ItemProperty $gamedvr -Name AppCaptureEnabled -ErrorAction SilentlyContinue }
  if ($null -ne $antes.GameDVR_Enabled) { Set-ItemProperty $config -Name GameDVR_Enabled -Value $antes.GameDVR_Enabled -Type DWord } else { Remove-ItemProperty $config -Name GameDVR_Enabled -ErrorAction SilentlyContinue }
  if ($null -ne $antes.AllowGameDVR) { New-ItemProperty $politica -Name AllowGameDVR -PropertyType DWord -Value $antes.AllowGameDVR -Force | Out-Null }
  "Depois: AppCaptureEnabled=$(Ler $gamedvr 'AppCaptureEnabled'), GameDVR_Enabled=$(Ler $config 'GameDVR_Enabled'), AllowGameDVR=$(Ler $politica 'AllowGameDVR'), serviços: " + (($servicos | ForEach-Object { "$_=$(Ler "HKLM:\SYSTEM\CurrentControlSet\Services\$_" 'Start')" }) -join ', ') + ' (devolvido)'
}
