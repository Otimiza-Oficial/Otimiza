# A otimização de janela do Windows (SwapEffectUpgradeEnable=1 no valor do jogo em UserGpuPreferences) tira um jogo
# do modelo de cópia? O jogo de teste apresenta como o FiveM em janela (OTIMIZA_TESTE_COPIA) no monitor 2, e o
# PresentMon diz o modo de apresentação de verdade, sem e com a opção. Precisa de administrador (o PresentMon lê o ETW).
# Devolve o registro como estava.
param(
  [string]$Exe = (Join-Path $PSScriptRoot '..\..\src-tauri\target\release\examples\jogo_de_teste.exe'),
  [int]$X = 1920,
  [int]$Largura = 1080,
  [int]$Altura = 1920
)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
  Write-Host 'Precisa de administrador: o PresentMon só mede assim. Abra o menu Iniciar, procure PowerShell,'
  Write-Host 'clique com o botão direito em "Executar como administrador" e rode este comando lá.'
  exit 1
}
$Exe = (Resolve-Path $Exe).Path
$pm = Join-Path $env:LOCALAPPDATA 'Otimiza\PresentMon.exe'
$chave = 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences'
$pasta = Join-Path $env:TEMP 'otimiza-lab-janela'
New-Item -ItemType Directory -Force $pasta | Out-Null
$env:OTIMIZA_TESTE_X = "$X"; $env:OTIMIZA_TESTE_Y = '0'; $env:OTIMIZA_TESTE_COPIA = '1'

function Medir($rotulo) {
  $jogo = Start-Process -FilePath $Exe -ArgumentList @('120', "$Largura", "$Altura", '0') -PassThru
  Start-Sleep -Seconds 3
  $csv = Join-Path $pasta "$rotulo.csv"
  $log = & $pm --process_name 'jogo_de_teste.exe' --timed 5 --terminate_after_timed --output_file $csv `
    --no_console_stats --session_name OtimizaLabJanela --stop_existing_session 2>&1
  Stop-Process -Id $jogo.Id -Force
  Start-Sleep -Milliseconds 500
  if (-not (Test-Path $csv)) {
    return "{0}: o PresentMon não gravou nada. Saída dele: {1}" -f $rotulo, (($log | Out-String).Trim())
  }
  $linhas = Import-Csv $csv
  $modos = $linhas | Group-Object PresentMode | Sort-Object Count -Descending | ForEach-Object { "$($_.Name) ($($_.Count))" }
  "{0}: {1}" -f $rotulo, ($modos -join ', ')
}

$antes = (Get-ItemProperty $chave -Name $Exe -ErrorAction SilentlyContinue).$Exe
try {
  Medir 'sem-a-otimizacao'
  Set-ItemProperty -Path $chave -Name $Exe -Value 'SwapEffectUpgradeEnable=1;'
  Medir 'com-a-otimizacao'
} finally {
  if ($antes) { Set-ItemProperty -Path $chave -Name $Exe -Value $antes } else { Remove-ItemProperty -Path $chave -Name $Exe -ErrorAction SilentlyContinue }
}
"Resultado também em $pasta"
