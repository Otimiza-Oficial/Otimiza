# Laboratório E1: a política AllowGameDVR=0 (gravada pelo ajuste "Desativar gravação em segundo plano (Game DVR)")
# bloqueia a Windows.Graphics.Capture? Testa a captura com a política como está, sem ela e devolve o valor de antes.
# Precisa de administrador (a política fica em HKLM).
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Write-Host 'Abra o PowerShell como administrador e rode de novo.'; exit 1 }

$chave = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\GameDVR'
$raiz = Join-Path $PSScriptRoot '..\..\src-tauri\target\release\examples'
$marcador = Join-Path $env:TEMP 'otimiza-wgc-marcador.txt'
$env:OTIMIZA_TESTE_X = '1280'; $env:OTIMIZA_TESTE_Y = '0'; $env:OTIMIZA_TESTE_COPIA = '1'; $env:OTIMIZA_TESTE_MARCADOR = $marcador

function Testar($rotulo) {
  $p = Start-Process -FilePath (Join-Path $raiz 'jogo_de_teste.exe') -ArgumentList @('60', '640', '360', '0') -PassThru
  Start-Sleep -Seconds 2
  $o = & (Join-Path $raiz 'bancada_captura.exe') wgc 5 $marcador 2>&1 | ForEach-Object { "$_" }
  Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 500
  $fim = ($o | Where-Object { $_ -match '^wgc \||Acesso negado' }) -join ' / '
  "{0}: {1}" -f $rotulo, $(if ($fim) { $fim } else { ($o | Select-Object -Last 2) -join ' / ' })
}

$antes = (Get-ItemProperty $chave -Name AllowGameDVR -ErrorAction SilentlyContinue).AllowGameDVR
"AllowGameDVR antes: $(if ($null -eq $antes) { '(sem a política)' } else { $antes })"
try {
  Testar 'com a política como está'
  Remove-ItemProperty $chave -Name AllowGameDVR -ErrorAction SilentlyContinue
  Testar 'sem a política AllowGameDVR'
} finally {
  if ($null -ne $antes) { New-ItemProperty $chave -Name AllowGameDVR -PropertyType DWord -Value $antes -Force | Out-Null }
  "AllowGameDVR depois: $((Get-ItemProperty $chave -Name AllowGameDVR -ErrorAction SilentlyContinue).AllowGameDVR) (devolvido)"
}
