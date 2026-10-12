// Laboratório E3: a MESMA bancada de captura de src-tauri/examples/bancada_captura.rs (método dxgi), em C++.
// Duplicação de Área de Trabalho como o Frame Boost (DuplicateOutput1 pedindo FP16 ou BGRA), lê o número do quadro
// que o jogo de teste escreve no canto e imprime a mesma linha de resultado, para comparar as linguagens.
//
//   compilar.cmd
//   bancada_captura.exe <segundos> <arquivo do marcador do jogo>
#define NOMINMAX
#include <windows.h>
#include <d3d11.h>
#include <dxgi1_6.h>
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <fstream>
#include <limits>
#include <string>
#include <unordered_map>
#include <vector>

#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "winmm.lib")

template <class T> struct Com {
    T* p = nullptr;
    ~Com() { if (p) p->Release(); }
    T** operator&() { return &p; }
    T* operator->() const { return p; }
    operator T*() const { return p; }
};

static int64_t qpc() { LARGE_INTEGER v; QueryPerformanceCounter(&v); return v.QuadPart; }

static double cpu_ms() {
    FILETIME c, s, k, u;
    GetProcessTimes(GetCurrentProcess(), &c, &s, &k, &u);
    auto v = [](FILETIME f) { return (static_cast<uint64_t>(f.dwHighDateTime) << 32) | f.dwLowDateTime; };
    return (v(k) + v(u)) / 10000.0;
}

int main(int argc, char** argv) {
    const double segundos = argc > 1 ? atof(argv[1]) : 10.0;
    const char* arquivo = argc > 2 ? argv[2] : nullptr;
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    timeBeginPeriod(1);
    LARGE_INTEGER freq; QueryPerformanceFrequency(&freq);

    HWND hwnd = FindWindowW(L"JogoDeTeste", nullptr);
    if (!hwnd) { std::printf("abra o jogo de teste antes\n"); return 1; }
    POINT origem{0, 0}; ClientToScreen(hwnd, &origem);
    HMONITOR monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);

    Com<IDXGIFactory1> fabrica; CreateDXGIFactory1(__uuidof(IDXGIFactory1), reinterpret_cast<void**>(&fabrica));
    Com<IDXGIAdapter1> adaptador; Com<IDXGIOutput5> saida; RECT area{};
    for (UINT i = 0; !saida.p && fabrica->EnumAdapters1(i, &adaptador.p) == S_OK; ++i) {
        IDXGIOutput* o = nullptr;
        for (UINT j = 0; adaptador->EnumOutputs(j, &o) == S_OK; ++j) {
            DXGI_OUTPUT_DESC d; o->GetDesc(&d);
            if (d.Monitor == monitor) { o->QueryInterface(&saida.p); area = d.DesktopCoordinates; o->Release(); break; }
            o->Release();
        }
        if (!saida.p) { adaptador.p->Release(); adaptador.p = nullptr; }
    }
    if (!saida.p) { std::printf("o monitor do jogo nao apareceu no DXGI\n"); return 1; }

    Com<ID3D11Device> device; Com<ID3D11DeviceContext> ctx;
    D3D_FEATURE_LEVEL niveis[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
    if (FAILED(D3D11CreateDevice(adaptador, D3D_DRIVER_TYPE_UNKNOWN, nullptr, D3D11_CREATE_DEVICE_BGRA_SUPPORT, niveis, 2,
                                 D3D11_SDK_VERSION, &device, nullptr, &ctx))) { std::printf("D3D11 falhou\n"); return 1; }

    DXGI_FORMAT formatos[] = {DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_B8G8R8A8_UNORM};
    Com<IDXGIOutputDuplication> dup;
    if (FAILED(saida->DuplicateOutput1(device, 0, 2, formatos, &dup))) { std::printf("duplicacao recusada\n"); return 1; }

    const UINT x = origem.x - area.left, y = origem.y - area.top;
    std::vector<std::pair<DXGI_FORMAT, ID3D11Texture2D*>> copias;
    auto ler_numero = [&](ID3D11Texture2D* t, uint32_t& n) -> bool {
        D3D11_TEXTURE2D_DESC d; t->GetDesc(&d);
        if (x + 160 > d.Width || y + 8 > d.Height) return false;
        ID3D11Texture2D* copia = nullptr;
        for (auto& c : copias) if (c.first == d.Format) copia = c.second;
        if (!copia) {
            D3D11_TEXTURE2D_DESC c{160, 8, 1, 1, d.Format, {1, 0}, D3D11_USAGE_STAGING, 0, D3D11_CPU_ACCESS_READ, 0};
            device->CreateTexture2D(&c, nullptr, &copia);
            copias.push_back({d.Format, copia});
        }
        D3D11_BOX caixa{x, y, 0, x + 160, y + 8, 1};
        ctx->CopySubresourceRegion(copia, 0, 0, 0, 0, t, 0, &caixa);
        D3D11_MAPPED_SUBRESOURCE m;
        if (FAILED(ctx->Map(copia, 0, D3D11_MAP_READ, 0, &m))) return false;
        const uint8_t* linha = static_cast<const uint8_t*>(m.pData) + 4 * m.RowPitch;
        n = 0;
        for (int bit = 0; bit < 20; ++bit) {
            const int px = bit * 8 + 4;
            bool claro;
            if (d.Format == DXGI_FORMAT_R16G16B16A16_FLOAT) {
                uint16_t h = linha[px * 8] | (linha[px * 8 + 1] << 8);
                claro = h > 0x3000;
            } else {
                claro = linha[px * 4] + linha[px * 4 + 1] + linha[px * 4 + 2] > 3 * 128;
            }
            if (claro) n |= 1u << bit;
        }
        ctx->Unmap(copia, 0);
        return true;
    };

    std::vector<std::pair<uint32_t, int64_t>> vistos;
    vistos.reserve(4096);
    const double cpu_antes = cpu_ms();
    const auto inicio = std::chrono::steady_clock::now();
    while (std::chrono::duration<double>(std::chrono::steady_clock::now() - inicio).count() < segundos) {
        DXGI_OUTDUPL_FRAME_INFO info{};
        IDXGIResource* recurso = nullptr;
        HRESULT hr = dup->AcquireNextFrame(100, &info, &recurso);
        if (hr == DXGI_ERROR_WAIT_TIMEOUT) continue;
        if (FAILED(hr)) { std::printf("a duplicacao parou: 0x%08X\n", static_cast<unsigned>(hr)); return 1; }
        const int64_t chegou = qpc();
        if (info.LastPresentTime.QuadPart != 0) {
            ID3D11Texture2D* t = nullptr;
            if (SUCCEEDED(recurso->QueryInterface(&t))) {
                uint32_t n;
                if (ler_numero(t, n)) vistos.push_back({n, chegou});
                t->Release();
            }
        }
        recurso->Release();
        dup->ReleaseFrame();
    }
    const double cpu = cpu_ms() - cpu_antes;

    std::unordered_map<uint32_t, int64_t> presentes;
    if (arquivo) {
        std::ifstream f(arquivo);
        uint32_t n; int64_t t;
        while (f >> n >> t) presentes[n] = t;
    }
    auto ms = [&](int64_t q) { return q * 1000.0 / freq.QuadPart; };
    std::vector<std::pair<uint32_t, int64_t>> novos;
    size_t repetidos = 0, lixo = 0;
    for (auto& v : vistos) {
        if (!presentes.count(v.first)) { ++lixo; continue; }
        if (!novos.empty() && novos.back().first >= v.first) ++repetidos; else novos.push_back(v);
    }
    const uint32_t primeiro = novos.empty() ? 0 : novos.front().first, ultimo = novos.empty() ? 0 : novos.back().first;
    size_t do_jogo = 0;
    for (auto& p : presentes) if (p.first >= primeiro && p.first <= ultimo) ++do_jogo;
    const size_t perdidos = do_jogo > novos.size() ? do_jogo - novos.size() : 0;
    const double duracao = novos.size() > 1 ? std::max(ms(novos.back().second - novos.front().second) / 1000.0, 1e-3) : 1e-3;
    std::vector<double> intervalos, atrasos;
    for (size_t i = 1; i < novos.size(); ++i) intervalos.push_back(ms(novos[i].second - novos[i - 1].second));
    for (auto& v : novos) atrasos.push_back(ms(v.second - presentes[v.first]));
    auto pct = [](std::vector<double>& v, double p) {
        if (v.empty()) return std::numeric_limits<double>::quiet_NaN();
        std::sort(v.begin(), v.end());
        return v[static_cast<size_t>((v.size() - 1) * p + 0.5)];
    };
    std::printf("dxgi-c++ | jogo %.1f fps | capturou %.1f/s novos (%zu quadros) | perdidos %zu (%.1f%%) | repetidos %zu | lixo %zu | "
                "intervalo p50 %.2f ms p99 %.2f ms max %.2f ms | atraso p50 %.2f ms p95 %.2f ms | CPU %.3f ms por quadro\n",
                do_jogo / duracao, novos.size() / duracao, novos.size(), perdidos, 100.0 * perdidos / std::max<size_t>(do_jogo, 1),
                repetidos, lixo, pct(intervalos, 0.5), pct(intervalos, 0.99), pct(intervalos, 1.0), pct(atrasos, 0.5),
                pct(atrasos, 0.95), cpu / std::max<size_t>(vistos.size(), 1));
    for (auto& c : copias) c.second->Release();
    timeEndPeriod(1);
    return 0;
}
