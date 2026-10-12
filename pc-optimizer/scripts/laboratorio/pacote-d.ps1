# Medição do pacote "d" (docs/app/ANALISE-PACOTES-DE-OTIMIZACAO.md): alterna os ajustes de desempenho que ele aplica
# entre o PADRÃO do Windows e os valores que o PC tinha (guardados na primeira vez). Não mexe em segurança.
# Precisa de administrador; reinicie depois de trocar (o agrupamento de serviços e o MMCSS só valem no boot).
#   powershell -ExecutionPolicy Bypass -File pacote-d.ps1 -Estado padrao     (desliga o pacote)
#   powershell -ExecutionPolicy Bypass -File pacote-d.ps1 -Estado pacote     (volta exatamente como estava)
param([Parameter(Mandatory = $true)][ValidateSet('padrao', 'pacote')][string]$Estado)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { Write-Host 'Abra o PowerShell como administrador e rode de novo.'; exit 1 }

$mm = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile'
# chave, nome, tipo, valor padrão do Windows ($null = o Windows não tem o valor)
$itens = @(
  @($mm, 'SystemResponsiveness', 'DWord', 20),
  @($mm, 'NetworkThrottlingIndex', 'DWord', 10),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\PriorityControl', 'Win32PrioritySeparation', 'DWord', 2),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling', 'PowerThrottlingOff', 'DWord', $null),
  @("$mm\Tasks\Games", 'GPU Priority', 'DWord', 8),
  @("$mm\Tasks\Games", 'Priority', 'DWord', 2),
  @("$mm\Tasks\Games", 'Scheduling Category', 'String', 'Medium'),
  @("$mm\Tasks\Games", 'SFIO Priority', 'String', 'Normal'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control', 'SvcHostSplitThresholdInKB', 'DWord', 3670016),
  @('HKLM:\SYSTEM\CurrentControlSet\Services\mouclass\Parameters', 'MouseDataQueueSize', 'DWord', $null)
)

$pasta = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Otimiza-laboratorio\medidas'
New-Item -ItemType Directory -Force $pasta | Out-Null
$copia = Join-Path $pasta 'pacote-d-valores-originais.json'
if (-not (Test-Path $copia)) {
  $originais = foreach ($i in $itens) {
    $v = (Get-ItemProperty -Path $i[0] -Name $i[1] -ErrorAction SilentlyContinue).($i[1])
    [pscustomobject]@{ Chave = $i[0]; Nome = $i[1]; Tipo = $i[2]; Valor = $v }
  }
  $originais | ConvertTo-Json | Set-Content -Path $copia -Encoding UTF8
  "Valores de hoje guardados em $copia"
}
$originais = Get-Content $copia -Raw | ConvertFrom-Json

foreach ($i in $itens) {
  $alvo = if ($Estado -eq 'padrao') { $i[3] } else { ($originais | Where-Object { $_.Chave -eq $i[0] -and $_.Nome -eq $i[1] }).Valor }
  if (-not (Test-Path $i[0])) { New-Item -Path $i[0] -Force | Out-Null }
  if ($null -eq $alvo) {
    Remove-ItemProperty -Path $i[0] -Name $i[1] -ErrorAction SilentlyContinue
  } else {
    # DWORD acima de 2147483647 (o 0xFFFFFFFF do NetworkThrottlingIndex) só entra como o mesmo número com sinal.
    if ($i[2] -eq 'DWord' -and [double]$alvo -gt 2147483647) { $alvo = [int]([double]$alvo - 4294967296) }
    New-ItemProperty -Path $i[0] -Name $i[1] -PropertyType $i[2] -Value $alvo -Force | Out-Null
  }
  $agora = (Get-ItemProperty -Path $i[0] -Name $i[1] -ErrorAction SilentlyContinue).($i[1])
  "{0,-28} = {1}" -f $i[1], $(if ($null -eq $agora) { '(padrao, sem valor)' } else { $agora })
}
"Estado: $Estado. Reinicie o PC antes de medir."
