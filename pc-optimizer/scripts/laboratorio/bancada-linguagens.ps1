# Laboratório E3: a mesma bancada de captura (Duplicação de Área de Trabalho, como o Frame Boost) em linguagens
# diferentes, revezando no mesmo jogo de teste. Não precisa de administrador.
#   powershell -ExecutionPolicy Bypass -File scripts\laboratorio\bancada-linguagens.ps1
param(
  [int[]]$Fps = @(60, 120),
  [int]$Segundos = 10,
  [int]$Repeticoes = 3,
  [int]$X = 1280, [int]$Y = 0, [int]$Largura = 640, [int]$Altura = 360
)
$base = Join-Path $PSScriptRoot '..\..'
$jogo = Join-Path $base 'src-tauri\target\release\examples\jogo_de_teste.exe'
# Cada linguagem: nome, executável e os argumentos antes de <segundos> <marcador>.
$linguagens = @(
  @{ Nome = 'Rust'; Exe = (Join-Path $base 'src-tauri\target\release\examples\bancada_captura.exe'); Args = @('dxgi') },
  @{ Nome = 'C++';  Exe = (Join-Path $base 'laboratorio\captura-cpp\bancada_captura.exe'); Args = @() }
)
foreach ($extra in @(
  @{ Nome = 'C#';  Exe = (Join-Path $base 'laboratorio\captura-cs\bin\publicado\bancada_captura.exe'); Args = @() },
  @{ Nome = 'Zig'; Exe = (Join-Path $base 'laboratorio\captura-zig\bancada_captura.exe'); Args = @() }
)) { if (Test-Path $extra.Exe) { $linguagens += $extra } }

$pasta = Join-Path $env:TEMP 'otimiza-bancada-linguagens'
New-Item -ItemType Directory -Force $pasta | Out-Null
foreach ($f in $Fps) {
  for ($r = 1; $r -le $Repeticoes; $r++) {
    foreach ($l in $linguagens) {
      $marcador = Join-Path $pasta "marcador-$f-$($l.Nome)-$r.txt"
      $env:OTIMIZA_TESTE_X = "$X"; $env:OTIMIZA_TESTE_Y = "$Y"; $env:OTIMIZA_TESTE_COPIA = '1'; $env:OTIMIZA_TESTE_MARCADOR = $marcador
      # Nenhum jogo de teste de rodada anterior pode sobrar: a bancada acha a janela pelo nome e mediria o jogo errado.
      Get-Process jogo_de_teste -ErrorAction SilentlyContinue | Stop-Process -Force
      Get-Process jogo_de_teste -ErrorAction SilentlyContinue | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue
      $p = Start-Process -FilePath $jogo -ArgumentList @("$f", "$Largura", "$Altura", '0') -PassThru -RedirectStandardOutput (Join-Path $pasta "jogo-$f-$($l.Nome)-$r.txt")
      Start-Sleep -Seconds 2
      $linha = & $l.Exe @($l.Args + @("$Segundos", $marcador)) 2>&1 | Select-Object -Last 1
      Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
      Wait-Process -Id $p.Id -Timeout 5 -ErrorAction SilentlyContinue
      Start-Sleep -Milliseconds 500
      "{0,-4} | jogo a {1} fps, rodada {2} | {3}" -f $l.Nome, $f, $r, $linha
    }
  }
}
Remove-Item Env:OTIMIZA_TESTE_MARCADOR, Env:OTIMIZA_TESTE_COPIA -ErrorAction SilentlyContinue
