# Laboratório E3: roda bancada-linguagens.ps1 e resume por linguagem com a MEDIANA das rodadas (uma rodada ruim de
# qualquer uma não decide). Não precisa de administrador.
param([int]$Repeticoes = 3)
$linhas = & (Join-Path $PSScriptRoot 'bancada-linguagens.ps1') -Repeticoes $Repeticoes 2>&1 | ForEach-Object { "$_" }
$linhas | ForEach-Object { $_ }
function Mediana($v) { $s = @($v | Sort-Object); if ($s.Count -eq 0) { return [double]::NaN }; $m = [int][Math]::Floor($s.Count / 2); if ($s.Count % 2) { $s[$m] } else { ($s[$m - 1] + $s[$m]) / 2 } }
$dados = foreach ($l in $linhas) {
  if ($l -match '^(\S+)\s+\| jogo a (\d+) fps.*?perdidos \d+ \(([\d.]+)%\).*?p99 ([\d.]+) ms max ([\d.]+) ms \| atraso p50 ([\d.]+) ms.*?CPU ([\d.]+) ms') {
    [pscustomobject]@{ L = $matches[1]; Fps = [int]$matches[2]; Perdidos = [double]$matches[3]; P99 = [double]$matches[4]; Max = [double]$matches[5]; Atraso = [double]$matches[6]; Cpu = [double]$matches[7] }
  }
}
''
'MEDIANAS ({0} rodadas por linguagem)' -f $Repeticoes
'{0,-5} {1,4} | {2,9} | {3,12} | {4,13} | {5,12} | {6,10}' -f 'ling.', 'fps', 'perdidos', 'p99 interv.', 'pior interv.', 'atraso p50', 'CPU/quadro'
foreach ($g in ($dados | Group-Object Fps, L)) {
  $v = $g.Group
  '{0,-5} {1,4} | {2,8:N1}% | {3,9:N1} ms | {4,10:N1} ms | {5,9:N2} ms | {6,7:N2} ms' -f $v[0].L, $v[0].Fps, (Mediana $v.Perdidos), (Mediana $v.P99), (Mediana $v.Max), (Mediana $v.Atraso), (Mediana $v.Cpu)
}
