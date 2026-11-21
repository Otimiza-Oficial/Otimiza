<#
.SYNOPSIS
  Mede o peso do próprio Otimiza (1.0.12, Frente 3.2; Onda 1 do PLANO-2.0-LEVE-E-RAPIDO): processador, memória,
  despertares, filhos e sessões ETW da árvore de processos do Otimiza, num período fixo.

.DESCRIPTION
  A árvore é o processo do Otimiza e todos os descendentes (WebView2, PresentMon, PowerShell, cmd...), relida a cada
  10 s para pegar filho novo. A cada amostra (padrão: 2 s):
    - processador: tempo de CPU gasto no intervalo, em % de UM núcleo (a régua das metas do plano 2.0). O Windows
      conta esse tempo em passos de ~15,6 ms: uma amostra de 2 s sai 0% ou 0,78%; vale a média de 60 s ou mais;
    - memória: conjunto de trabalho e memória privada somados;
    - despertares: trocas de contexto por segundo das threads da árvore (contador "\Thread(*)\Context Switches/sec").
      É um teto: conta também o acordar por E/S e por mensagem de janela, não só os temporizadores do Otimiza.
  No começo e no fim: as sessões ETW abertas com "Otimiza" ou "PresentMon" no nome (`logman query -ets`, pede
  administrador; sem ele fica "não lido").

  Nada é escrito no Windows. O próprio script custa processador (o contador de threads lê todas as threads do PC);
  esse custo cai no powershell.exe que roda o script, que não faz parte da árvore medida.

  Para comparar duas versões (A-B): mesma máquina, mesmo jogo, mesma cena, o mesmo -Segundos, e pelo menos três
  rodadas de cada lado, alternando A e B. Um número só vai para nota de versão medido assim.

.PARAMETER Segundos
  Duração da medição. Padrão 120 (a meta de CPU do plano é a média de 60 s; o resto dá folga para o começo).

.PARAMETER Intervalo
  Segundos entre amostras. Padrão 2.

.PARAMETER Processo
  Nome do executável do Otimiza, sem ".exe". Padrão "pc-optimizer" (o nome do binário do Tauri).

.PARAMETER Saida
  Pasta onde ficam o CSV das amostras e o resumo em JSON. Padrão: medidas-peso\<data-hora> na pasta atual.

.PARAMETER Rotulo
  Texto livre gravado no resumo (ex.: "1.0.12 FiveM jogo aberto sem Frame Boost").

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\medir-peso.ps1 -Segundos 120 -Rotulo "1.0.12 FiveM sem FB"
#>
[CmdletBinding()]
param(
  [ValidateRange(10, 3600)][int]$Segundos = 120,
  [ValidateRange(1, 60)][int]$Intervalo = 2,
  [ValidatePattern('^[A-Za-z0-9._-]+$')][string]$Processo = "pc-optimizer",
  [string]$Saida = "",
  [string]$Rotulo = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 3

# As metas da seção 1 do PLANO-2.0-LEVE-E-RAPIDO (a 2.0), para o resumo dizer onde cada número está.
$META_CPU_PCT_NUCLEO = 0.5
$META_MEMORIA_MB = 200
$META_DESPERTARES_MIN = 30
$META_SESSOES_ETW = 1

function Ler-Arvore([int]$raiz) {
  # Uma consulta só ao WMI por releitura: a lista inteira de processos, e a descida pelos pais fica aqui.
  $todos = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, CreationDate
  $filhos = @{}
  foreach ($p in $todos) {
    $pai = [int]$p.ParentProcessId
    if (-not $filhos.ContainsKey($pai)) { $filhos[$pai] = New-Object System.Collections.Generic.List[object] }
    $filhos[$pai].Add($p)
  }
  $arvore = @{}
  $raizInfo = $todos | Where-Object { [int]$_.ProcessId -eq $raiz } | Select-Object -First 1
  if (-not $raizInfo) { return $arvore }
  $fila = New-Object System.Collections.Generic.Queue[object]
  $fila.Enqueue($raizInfo)
  while ($fila.Count -gt 0) {
    $p = $fila.Dequeue()
    $id = [int]$p.ProcessId
    if ($arvore.ContainsKey($id)) { continue }
    $arvore[$id] = [pscustomobject]@{ Pid = $id; Nome = [string]$p.Name; Criado = $p.CreationDate }
    if ($filhos.ContainsKey($id)) {
      # Um PID reaproveitado pode ter um "pai" que nasceu depois dele: só entra filho criado depois do pai.
      foreach ($f in $filhos[$id]) {
        if ($f.CreationDate -ge $p.CreationDate) { $fila.Enqueue($f) }
      }
    }
  }
  return $arvore
}

function Ler-SessoesEtw {
  try {
    $saida = & logman.exe query -ets 2>$null
    if ($LASTEXITCODE -ne 0) { return $null }
    return @($saida | Where-Object { $_ -match '(?i)otimiza|presentmon' } | ForEach-Object { ($_ -split '\s{2,}')[0].Trim() })
  } catch {
    return $null
  }
}

function Percentil([double[]]$valores, [double]$p) {
  if ($valores.Count -eq 0) { return $null }
  $ordenados = $valores | Sort-Object
  $i = [math]::Ceiling($p * $ordenados.Count) - 1
  return $ordenados[[math]::Max(0, [math]::Min($i, $ordenados.Count - 1))]
}

$principal = @(Get-Process -Name $Processo -ErrorAction SilentlyContinue | Sort-Object StartTime)
if ($principal.Count -eq 0) {
  Write-Error "O Otimiza ($Processo.exe) não está aberto. Abra o Otimiza (e o jogo, se for medir com ele) antes."
}
# O primeiro aberto é o processo do app; uma segunda instância seria filha dele ou um `--restaurar-tudo`.
$raiz = $principal[0].Id

if (-not $Saida) { $Saida = Join-Path (Get-Location) ("medidas-peso\" + (Get-Date -Format "yyyyMMdd-HHmmss")) }
New-Item -ItemType Directory -Force -Path $Saida | Out-Null
$csv = Join-Path $Saida "amostras.csv"
$resumoArquivo = Join-Path $Saida "resumo.json"

$nucleos = [Environment]::ProcessorCount
$versao = try { $principal[0].MainModule.FileVersionInfo.ProductVersion } catch { $null }
$sessoesAntes = Ler-SessoesEtw

$arvore = Ler-Arvore $raiz
$vistos = @{}
foreach ($k in $arvore.Keys) { $vistos[$k] = $arvore[$k].Nome }
$filhosNovos = New-Object System.Collections.Generic.List[string]
$cpuAnterior = @{}
foreach ($k in $arvore.Keys) {
  try { $cpuAnterior[$k] = (Get-Process -Id $k).TotalProcessorTime.TotalMilliseconds } catch { }
}
$categoriaThread = New-Object System.Diagnostics.PerformanceCounterCategory("Thread")
$trocasAnteriores = @{}
$relogio = [Diagnostics.Stopwatch]::StartNew()
$ultimoTempo = 0.0
$ultimaArvore = 0.0
$amostras = New-Object System.Collections.Generic.List[object]

Write-Host "Medindo a árvore do $Processo (PID $raiz, $($arvore.Count) processos) por $Segundos s..."
while ($relogio.Elapsed.TotalSeconds -lt $Segundos) {
  Start-Sleep -Seconds $Intervalo
  $agora = $relogio.Elapsed.TotalSeconds

  if ($agora - $ultimaArvore -ge 10) {
    $arvore = Ler-Arvore $raiz
    $ultimaArvore = $agora
    if ($arvore.Count -eq 0) { Write-Warning "O Otimiza fechou durante a medição."; break }
    foreach ($k in $arvore.Keys) {
      if (-not $vistos.ContainsKey($k)) {
        $vistos[$k] = $arvore[$k].Nome
        $filhosNovos.Add("$($arvore[$k].Nome) (PID $k, aos $([math]::Round($agora)) s)")
      }
    }
  }

  $cpuMs = 0.0; $ws = 0.0; $privada = 0.0; $vivos = 0; $powershells = 0
  foreach ($k in @($arvore.Keys)) {
    try { $p = Get-Process -Id $k } catch { continue }
    $vivos++
    if ($p.ProcessName -match '^(powershell|pwsh)$') { $powershells++ }
    $total = $p.TotalProcessorTime.TotalMilliseconds
    # Processo que apareceu agora: o tempo dele antes de entrar na árvore não conta para este intervalo.
    if ($cpuAnterior.ContainsKey($k)) { $cpuMs += [math]::Max(0, $total - $cpuAnterior[$k]) }
    $cpuAnterior[$k] = $total
    $ws += $p.WorkingSet64
    $privada += $p.PrivateMemorySize64
  }

  # Trocas de contexto: o contador bruto (acumulado) de cada thread, com o PID dela; soma o quanto andou só nas
  # threads da árvore. Pelo .NET e com o nome em inglês: o `Get-Counter` só aceita o nome traduzido, e o Windows dos
  # clientes está em português.
  $despertares = $null
  try {
    $dados = $categoriaThread.ReadCategory()
    $pids = $dados["ID Process"]
    $trocas = $dados["Context Switches/sec"]
    $atual = @{}
    $soma = 0.0
    foreach ($inst in $trocas.Keys) {
      $tp = [int]$pids[$inst].RawValue
      if (-not $arvore.ContainsKey($tp)) { continue }
      $chave = "$tp|$inst"
      $bruto = [double]$trocas[$inst].RawValue
      $atual[$chave] = $bruto
      # Thread nova (ou instância reaproveitada) entra na próxima amostra.
      if ($trocasAnteriores.ContainsKey($chave) -and $bruto -ge $trocasAnteriores[$chave]) { $soma += $bruto - $trocasAnteriores[$chave] }
    }
    $trocasAnteriores = $atual
    $despertares = $soma
  } catch { $despertares = $null }

  $dt = $agora - $ultimoTempo
  $ultimoTempo = $agora
  $amostras.Add([pscustomobject]@{
    segundo = [math]::Round($agora, 1)
    processos = $vivos
    powershell = $powershells
    cpu_pct_de_um_nucleo = [math]::Round($cpuMs / ($dt * 1000) * 100, 3)
    memoria_ws_mb = [math]::Round($ws / 1MB, 1)
    memoria_privada_mb = [math]::Round($privada / 1MB, 1)
    trocas_de_contexto_por_s = if ($null -eq $despertares) { $null } else { [math]::Round($despertares / $dt, 1) }
  })
}

$amostras | Export-Csv -Path $csv -NoTypeInformation -Encoding UTF8
$sessoesDepois = Ler-SessoesEtw

# A primeira amostra pega o intervalo de partida inteiro: fica fora das médias.
$validas = @($amostras | Select-Object -Skip 1)
if ($validas.Count -eq 0) { Write-Error "Nenhuma amostra válida: aumente -Segundos." }
$cpu = [double[]]@($validas | ForEach-Object { $_.cpu_pct_de_um_nucleo })
$memoria = [double[]]@($validas | ForEach-Object { $_.memoria_privada_mb })
$cs = [double[]]@($validas | Where-Object { $null -ne $_.trocas_de_contexto_por_s } | ForEach-Object { $_.trocas_de_contexto_por_s })
$cpuMedia = ($cpu | Measure-Object -Average).Average
$memMax = ($memoria | Measure-Object -Maximum).Maximum
$csMedia = if ($cs.Count) { ($cs | Measure-Object -Average).Average } else { $null }

$resumo = [ordered]@{
  rotulo = $Rotulo
  quando = (Get-Date).ToString("s")
  versao_do_otimiza = $versao
  processo = "$Processo (PID $raiz)"
  nucleos_logicos = $nucleos
  segundos = [math]::Round($relogio.Elapsed.TotalSeconds)
  amostras_validas = $validas.Count
  cpu_pct_de_um_nucleo = [ordered]@{ media = [math]::Round($cpuMedia, 3); p95 = (Percentil $cpu 0.95); maximo = ($cpu | Measure-Object -Maximum).Maximum; meta_2_0 = $META_CPU_PCT_NUCLEO; dentro = ($cpuMedia -lt $META_CPU_PCT_NUCLEO) }
  memoria_privada_mb = [ordered]@{ maximo = $memMax; meta_2_0 = $META_MEMORIA_MB; dentro = ($memMax -lt $META_MEMORIA_MB) }
  trocas_de_contexto_por_minuto = if ($null -eq $csMedia) { "não lido" } else { [ordered]@{ media = [math]::Round($csMedia * 60); meta_2_0_despertares = $META_DESPERTARES_MIN; observacao = "teto: conta todo acordar das threads, não só os temporizadores" } }
  processos_novos_na_arvore = @($filhosNovos)
  powershell_na_arvore_maximo = ($amostras | Measure-Object -Property powershell -Maximum).Maximum
  sessoes_etw = [ordered]@{ antes = if ($null -eq $sessoesAntes) { "não lido (precisa de administrador)" } else { @($sessoesAntes) }; depois = if ($null -eq $sessoesDepois) { "não lido (precisa de administrador)" } else { @($sessoesDepois) }; meta_2_0 = $META_SESSOES_ETW }
  amostras_csv = $csv
}
$resumo | ConvertTo-Json -Depth 5 | Out-File -FilePath $resumoArquivo -Encoding utf8
$resumo | ConvertTo-Json -Depth 5
Write-Host "Resumo em $resumoArquivo"
