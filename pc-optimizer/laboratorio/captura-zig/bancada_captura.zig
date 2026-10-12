// Laboratório E3: a MESMA bancada de captura de src-tauri/examples/bancada_captura.rs (método dxgi), em Zig.
// O DirectX vem dos cabeçalhos do Windows que o Zig traz (@cImport), chamado pela vtable como em C. Para não depender
// da biblioteca padrão do Zig (que muda muito entre versões), argumentos, arquivo e ordenação vão pelo Windows e pelo C.
//
//   ..\ferramentas\zig-x86_64-windows-0.16.0\zig.exe build-exe bancada_captura.zig -O ReleaseFast -lc -ld3d11 -ldxgi -lwinmm -lshell32 -luser32
//   bancada_captura.exe <segundos> <arquivo do marcador do jogo>
const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", "1");
    @cInclude("windows.h");
    @cInclude("shellapi.h");
    @cInclude("timeapi.h");
    @cInclude("share.h");
    @cInclude("d3d11.h");
    @cInclude("dxgi1_6.h");
    @cInclude("stdio.h");
    @cInclude("stdlib.h");
    @cInclude("wchar.h");
});

fn guid(d1: u32, d2: u16, d3: u16, d4: [8]u8) c.GUID {
    return .{ .Data1 = d1, .Data2 = d2, .Data3 = d3, .Data4 = d4 };
}

fn qpc() i64 {
    var v: c.LARGE_INTEGER = undefined;
    _ = c.QueryPerformanceCounter(&v);
    return v.QuadPart;
}

fn cpuMs() f64 {
    var cr: c.FILETIME = undefined;
    var sa: c.FILETIME = undefined;
    var k: c.FILETIME = undefined;
    var u: c.FILETIME = undefined;
    _ = c.GetProcessTimes(c.GetCurrentProcess(), &cr, &sa, &k, &u);
    const kk = (@as(u64, k.dwHighDateTime) << 32) | k.dwLowDateTime;
    const uu = (@as(u64, u.dwHighDateTime) << 32) | u.dwLowDateTime;
    return @as(f64, @floatFromInt(kk + uu)) / 10000.0;
}

fn comparar(a: ?*const anyopaque, b: ?*const anyopaque) callconv(.c) c_int {
    const x = @as(*const f64, @ptrCast(@alignCast(a))).*;
    const y = @as(*const f64, @ptrCast(@alignCast(b))).*;
    return if (x < y) -1 else if (x > y) 1 else 0;
}

fn pct(v: []f64, p: f64) f64 {
    if (v.len == 0) return 0.0 / @as(f64, 0.0);
    c.qsort(v.ptr, v.len, @sizeOf(f64), comparar);
    const i: usize = @intFromFloat(@round(@as(f64, @floatFromInt(v.len - 1)) * p));
    return v[i];
}

const Visto = struct { n: u32, t: i64 };
var vistos: [200000]Visto = undefined;
var novos: [200000]Visto = undefined;
var intervalos: [200000]f64 = undefined;
var atrasos: [200000]f64 = undefined;

pub fn main() u8 {
    var argc: c_int = 0;
    const argv = c.CommandLineToArgvW(c.GetCommandLineW(), &argc);
    const segundos: f64 = if (argc > 1) c.wcstod(argv[1], null) else 10.0;
    const arquivo: ?[*c]u16 = if (argc > 2) argv[2] else null;

    _ = c.SetProcessDpiAwarenessContext(@ptrFromInt(@as(usize, @bitCast(@as(isize, -4)))));
    _ = c.timeBeginPeriod(1);
    var freq: c.LARGE_INTEGER = undefined;
    _ = c.QueryPerformanceFrequency(&freq);

    const classe = [_:0]u16{ 'J', 'o', 'g', 'o', 'D', 'e', 'T', 'e', 's', 't', 'e' };
    const hwnd = c.FindWindowW(&classe, null);
    if (hwnd == null) {
        _ = c.printf("abra o jogo de teste antes\n");
        return 1;
    }
    var origem = c.POINT{ .x = 0, .y = 0 };
    _ = c.ClientToScreen(hwnd, &origem);
    const monitor = c.MonitorFromWindow(hwnd, c.MONITOR_DEFAULTTONEAREST);

    var iid_fabrica = guid(0x770aae78, 0xf26f, 0x4dba, .{ 0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87 });
    var iid_saida5 = guid(0x80a07424, 0xab52, 0x42eb, .{ 0x83, 0x3c, 0x0c, 0x42, 0xfd, 0x28, 0x2d, 0x98 });
    var iid_textura = guid(0x6f15aaf2, 0xd208, 0x4e89, .{ 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f, 0x9c });

    var fabrica: ?*c.IDXGIFactory1 = null;
    _ = c.CreateDXGIFactory1(&iid_fabrica, @ptrCast(&fabrica));
    var adaptador: ?*c.IDXGIAdapter1 = null;
    var saida5: ?*c.IDXGIOutput5 = null;
    var area: c.RECT = undefined;
    var i: c_uint = 0;
    while (saida5 == null) : (i += 1) {
        var a: ?*c.IDXGIAdapter1 = null;
        if (fabrica.?.lpVtbl.*.EnumAdapters1.?(fabrica, i, &a) != 0) break;
        var j: c_uint = 0;
        while (true) : (j += 1) {
            var o: ?*c.IDXGIOutput = null;
            if (a.?.lpVtbl.*.EnumOutputs.?(@ptrCast(a), j, &o) != 0) break;
            var d: c.DXGI_OUTPUT_DESC = undefined;
            _ = o.?.lpVtbl.*.GetDesc.?(o, &d);
            if (d.Monitor == monitor) {
                _ = o.?.lpVtbl.*.QueryInterface.?(o, &iid_saida5, @ptrCast(&saida5));
                area = d.DesktopCoordinates;
                adaptador = a;
            }
            _ = o.?.lpVtbl.*.Release.?(o);
            if (saida5 != null) break;
        }
        if (saida5 == null) _ = a.?.lpVtbl.*.Release.?(a);
    }
    if (saida5 == null) {
        _ = c.printf("o monitor do jogo nao apareceu no DXGI\n");
        return 1;
    }

    var device: ?*c.ID3D11Device = null;
    var ctx: ?*c.ID3D11DeviceContext = null;
    var niveis = [_]c.D3D_FEATURE_LEVEL{ c.D3D_FEATURE_LEVEL_11_1, c.D3D_FEATURE_LEVEL_11_0 };
    if (c.D3D11CreateDevice(@ptrCast(adaptador), c.D3D_DRIVER_TYPE_UNKNOWN, null, c.D3D11_CREATE_DEVICE_BGRA_SUPPORT, &niveis, 2, c.D3D11_SDK_VERSION, &device, null, &ctx) != 0) {
        _ = c.printf("D3D11 falhou\n");
        return 1;
    }
    var formatos = [_]c.DXGI_FORMAT{ c.DXGI_FORMAT_R16G16B16A16_FLOAT, c.DXGI_FORMAT_B8G8R8A8_UNORM };
    var dup: ?*c.IDXGIOutputDuplication = null;
    if (saida5.?.lpVtbl.*.DuplicateOutput1.?(saida5, @ptrCast(device), 0, 2, &formatos, &dup) != 0) {
        _ = c.printf("duplicacao recusada\n");
        return 1;
    }

    const x: c_uint = @intCast(origem.x - area.left);
    const y: c_uint = @intCast(origem.y - area.top);
    var copia_fmt: c.DXGI_FORMAT = 0;
    var copia: ?*c.ID3D11Texture2D = null;

    var total: usize = 0;
    const cpu_antes = cpuMs();
    const inicio = qpc();
    const fim = inicio + @as(i64, @intFromFloat(segundos * @as(f64, @floatFromInt(freq.QuadPart))));
    while (qpc() < fim) {
        var info: c.DXGI_OUTDUPL_FRAME_INFO = undefined;
        var recurso: ?*c.IDXGIResource = null;
        const hr = dup.?.lpVtbl.*.AcquireNextFrame.?(dup, 100, &info, &recurso);
        if (hr == @as(c.HRESULT, @bitCast(@as(u32, 0x887A0027)))) continue;
        if (hr != 0) {
            _ = c.printf("a duplicacao parou: 0x%08X\n", @as(c_uint, @bitCast(hr)));
            return 1;
        }
        const chegou = qpc();
        if (info.LastPresentTime.QuadPart != 0) {
            var t: ?*c.ID3D11Texture2D = null;
            if (recurso.?.lpVtbl.*.QueryInterface.?(recurso, &iid_textura, @ptrCast(&t)) == 0) {
                var d: c.D3D11_TEXTURE2D_DESC = undefined;
                t.?.lpVtbl.*.GetDesc.?(t, &d);
                if (x + 160 <= d.Width and y + 8 <= d.Height) {
                    if (copia == null or copia_fmt != d.Format) {
                        if (copia != null) _ = copia.?.lpVtbl.*.Release.?(copia);
                        var cd = c.D3D11_TEXTURE2D_DESC{ .Width = 160, .Height = 8, .MipLevels = 1, .ArraySize = 1, .Format = d.Format, .SampleDesc = .{ .Count = 1, .Quality = 0 }, .Usage = c.D3D11_USAGE_STAGING, .BindFlags = 0, .CPUAccessFlags = c.D3D11_CPU_ACCESS_READ, .MiscFlags = 0 };
                        _ = device.?.lpVtbl.*.CreateTexture2D.?(device, &cd, null, &copia);
                        copia_fmt = d.Format;
                    }
                    var caixa = c.D3D11_BOX{ .left = x, .top = y, .front = 0, .right = x + 160, .bottom = y + 8, .back = 1 };
                    ctx.?.lpVtbl.*.CopySubresourceRegion.?(ctx, @ptrCast(copia), 0, 0, 0, 0, @ptrCast(t), 0, &caixa);
                    var m: c.D3D11_MAPPED_SUBRESOURCE = undefined;
                    if (ctx.?.lpVtbl.*.Map.?(ctx, @ptrCast(copia), 0, c.D3D11_MAP_READ, 0, &m) == 0) {
                        const linha: [*]const u8 = @as([*]const u8, @ptrCast(m.pData)) + 4 * m.RowPitch;
                        var n: u32 = 0;
                        var bit: u5 = 0;
                        while (bit < 20) : (bit += 1) {
                            const px: usize = @as(usize, bit) * 8 + 4;
                            const claro = if (d.Format == c.DXGI_FORMAT_R16G16B16A16_FLOAT)
                                (@as(u16, linha[px * 8]) | (@as(u16, linha[px * 8 + 1]) << 8)) > 0x3000
                            else
                                @as(u32, linha[px * 4]) + linha[px * 4 + 1] + linha[px * 4 + 2] > 3 * 128;
                            if (claro) n |= @as(u32, 1) << bit;
                        }
                        ctx.?.lpVtbl.*.Unmap.?(ctx, @ptrCast(copia), 0);
                        if (total < vistos.len) {
                            vistos[total] = .{ .n = n, .t = chegou };
                            total += 1;
                        }
                    }
                }
                _ = t.?.lpVtbl.*.Release.?(t);
            }
        }
        _ = recurso.?.lpVtbl.*.Release.?(recurso);
        _ = dup.?.lpVtbl.*.ReleaseFrame.?(dup);
    }
    const cpu = cpuMs() - cpu_antes;

    // O que o jogo apresentou: "número instante" por linha. Os números começam em 1 e sobem de 1 em 1.
    const presentes: [*]i64 = @ptrCast(@alignCast(c.calloc(2_000_000, @sizeOf(i64))));
    var maior: u32 = 0;
    if (arquivo) |caminho| {
        const modo = [_:0]u16{ 'r', 0 };
        // _wfsopen com compartilhamento: o jogo ainda está escrevendo no arquivo.
        const f = c._wfsopen(caminho, &modo, c._SH_DENYNO);
        if (f != null) {
            var n: c_uint = 0;
            var q: c_longlong = 0;
            while (c.fscanf(f, "%u %lld", &n, &q) == 2) {
                if (n < 2_000_000) {
                    presentes[n] = q;
                    if (n > maior) maior = n;
                }
            }
            _ = c.fclose(f);
        }
    }
    const ms = struct {
        fn de(q: i64, f: i64) f64 {
            return @as(f64, @floatFromInt(q)) * 1000.0 / @as(f64, @floatFromInt(f));
        }
    }.de;
    var quantos_novos: usize = 0;
    var repetidos: usize = 0;
    var lixo: usize = 0;
    for (vistos[0..total]) |v| {
        if (v.n == 0 or v.n > maior or presentes[v.n] == 0) {
            lixo += 1;
            continue;
        }
        if (quantos_novos > 0 and novos[quantos_novos - 1].n >= v.n) {
            repetidos += 1;
        } else {
            novos[quantos_novos] = v;
            quantos_novos += 1;
        }
    }
    var do_jogo: usize = 0;
    if (quantos_novos > 0) {
        var k = novos[0].n;
        while (k <= novos[quantos_novos - 1].n) : (k += 1) {
            if (presentes[k] != 0) do_jogo += 1;
        }
    }
    const perdidos = if (do_jogo > quantos_novos) do_jogo - quantos_novos else 0;
    const duracao = if (quantos_novos > 1) @max(ms(novos[quantos_novos - 1].t - novos[0].t, freq.QuadPart) / 1000.0, 1e-3) else 1e-3;
    var ni: usize = 0;
    var k2: usize = 1;
    while (k2 < quantos_novos) : (k2 += 1) {
        intervalos[ni] = ms(novos[k2].t - novos[k2 - 1].t, freq.QuadPart);
        ni += 1;
    }
    for (novos[0..quantos_novos], 0..) |v, idx| atrasos[idx] = ms(v.t - presentes[v.n], freq.QuadPart);
    const fdj: f64 = @floatFromInt(do_jogo);
    const fnv: f64 = @floatFromInt(quantos_novos);
    _ = c.printf("dxgi-zig | jogo %.1f fps | capturou %.1f/s novos (%zu quadros) | perdidos %zu (%.1f%%) | repetidos %zu | lixo %zu | intervalo p50 %.2f ms p99 %.2f ms max %.2f ms | atraso p50 %.2f ms p95 %.2f ms | CPU %.3f ms por quadro\n",
        fdj / duracao, fnv / duracao, quantos_novos, perdidos, 100.0 * @as(f64, @floatFromInt(perdidos)) / @max(fdj, 1.0),
        repetidos, lixo, pct(intervalos[0..ni], 0.5), pct(intervalos[0..ni], 0.99), pct(intervalos[0..ni], 1.0),
        pct(atrasos[0..quantos_novos], 0.5), pct(atrasos[0..quantos_novos], 0.95), cpu / @max(@as(f64, @floatFromInt(total)), 1.0));
    _ = c.timeEndPeriod(1);
    return 0;
}
