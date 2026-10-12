// Laboratório E1/E3: a Windows.Graphics.Capture em C++/WinRT, para comparar com a versão em Rust
// (src-tauri/examples/bancada_captura.rs). Primeiro passo: a captura abre neste PC? Captura o monitor principal por
// 3 segundos e diz quantos quadros chegaram, ou o erro exato.
//
//   compilar.cmd   (usa o MSVC das Build Tools; runtime embutido com /MT, sem pedir o Visual C++ Redistributable)
#include <windows.h>
#include <d3d11.h>
#include <dxgi.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>
#include <winrt/Windows.Security.Authorization.AppCapabilityAccess.h>
#include <utility>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <thread>

#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "windowsapp.lib")

using namespace winrt;
using namespace winrt::Windows::Graphics::Capture;
using namespace winrt::Windows::Graphics::DirectX;
using namespace winrt::Windows::Graphics::DirectX::Direct3D11;

int main(int argc, char** argv) {
    init_apartment(apartment_type::multi_threaded);
    const bool janela = argc > 1 && std::string(argv[1]) == "janela";
    try {
        std::printf("suportada: %d\n", GraphicsCaptureSession::IsSupported() ? 1 : 0);
        // Windows 11 24H2+: capturar janela ou monitor sem a pessoa escolher (sem o seletor) e sem a borda amarela
        // exige pedir antes. Programas comuns recebem na hora.
        for (auto [tipo, nome] : {std::pair{GraphicsCaptureAccessKind::Programmatic, "programatica"},
                                  std::pair{GraphicsCaptureAccessKind::Borderless, "sem borda"}}) {
            auto r = GraphicsCaptureAccess::RequestAccessAsync(tipo).get();
            std::printf("acesso %s: %d\n", nome, static_cast<int>(r));
        }

        com_ptr<ID3D11Device> d3d;
        check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                                        nullptr, 0, D3D11_SDK_VERSION, d3d.put(), nullptr, nullptr));
        auto dxgi = d3d.as<IDXGIDevice>();
        com_ptr<::IInspectable> inspecionavel;
        check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgi.get(), inspecionavel.put()));
        auto dispositivo = inspecionavel.as<IDirect3DDevice>();

        auto interop = get_activation_factory<GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
        GraphicsCaptureItem item{nullptr};
        if (janela) {
            HWND hwnd = FindWindowW(L"JogoDeTeste", nullptr);
            if (!hwnd) { std::printf("abra o jogo de teste\n"); return 1; }
            check_hresult(interop->CreateForWindow(hwnd, guid_of<GraphicsCaptureItem>(), put_abi(item)));
        } else {
            HMONITOR monitor = MonitorFromPoint({0, 0}, MONITOR_DEFAULTTOPRIMARY);
            check_hresult(interop->CreateForMonitor(monitor, guid_of<GraphicsCaptureItem>(), put_abi(item)));
        }

        auto pool = Direct3D11CaptureFramePool::CreateFreeThreaded(dispositivo, DirectXPixelFormat::B8G8R8A8UIntNormalized,
                                                                   1, item.Size());
        std::atomic<int> quadros{0};
        pool.FrameArrived([&](auto const& p, auto const&) {
            if (auto q = p.TryGetNextFrame()) { ++quadros; q.Close(); }
        });
        auto sessao = pool.CreateCaptureSession(item);
        sessao.StartCapture();
        std::this_thread::sleep_for(std::chrono::seconds(3));
        sessao.Close();
        pool.Close();
        std::printf("%s: %d quadros em 3 s\n", janela ? "janela" : "monitor", quadros.load());
        return 0;
    } catch (hresult_error const& e) {
        std::printf("erro 0x%08X: %ls\n", static_cast<unsigned>(e.code()), e.message().c_str());
        return 2;
    }
}
