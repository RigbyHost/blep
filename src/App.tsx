
import { useState, useEffect } from "react";

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Sidebar, SidebarContent, SidebarFooter, SidebarProvider } from "./components/ui/sidebar";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Card } from "./components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "./components/ui/dialog";
import { Label } from "./components/ui/label";
import { getGDPS, GDPS } from "./api/getGDPS";

function App() {
  const [servers, setServers] = useState<GDPS[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [newServerUrl, setNewServerUrl] = useState("");
  const [patching, setPatching] = useState(false);
  const [patchStatus, setPatchStatus] = useState("");
  const [selectedServer, setSelectedServer] = useState<GDPS | null>(null);

  useEffect(() => {
    fetchServers();
  }, [])

  async function fetchServers() {
    try {
      const serverIds = await invoke<string[]>("scan_servers");
      const serverPromises = serverIds.map(id => getGDPS(id));
      const serverData = await Promise.all(serverPromises);
      setServers(serverData.filter(s => s.success));
    } catch (e) {
      console.error("Failed to scan servers:", e);
    }
  }

  async function handleAddServer() {
    if (!newServerUrl.trim()) {
      alert("Неверный ID");
      return;
    }
    setPatching(true);
    setPatchStatus("Патчим игру...");
    try {
      await invoke("patch_game", { id: newServerUrl.trim() });
      setPatchStatus("Готово!");
      setNewServerUrl("");
      setShowAddModal(false);
      fetchServers();
    } catch (e) {
      setPatchStatus(`Ошибка: ${e} `);
      alert(`Не удалось пропатчить: ${e} `);
    } finally {
      setPatching(false);
    }
  }

  async function handleRunGame() {
    if (!selectedServer) return;
    try {
      await invoke("run_game", { id: selectedServer.server.srvid });
    } catch (e) {
      alert(`Ошибка запуска: ${e}`);
    }
  }

  function handleClose() {
    getCurrentWindow().close();
  }

  function handleMinimize() {
    getCurrentWindow().minimize();
  }

  return (
    <SidebarProvider>
      {selectedServer && (
        <div
          className="fixed inset-0 w-full h-full z-0 bg-cover bg-center transition-all duration-700 ease-in-out"
          style={{
            backgroundImage: `url(${selectedServer.server.backgroundImage})`,
          }}
        >
          <div className="absolute inset-0 bg-black/40 backdrop-blur-[2px]" />
        </div>
      )}
      <Sidebar>
        <SidebarContent className="py-2">
          <div
            className="flex items-center gap-3 px-3 py-2 cursor-default select-none"
            data-tauri-drag-region
          >
            <div className="flex gap-2">
              <button
                onClick={handleClose}
                className="w-3 h-3 rounded-full bg-red-500 hover:bg-red-600 transition-colors"
                aria-label="Close"
              />
              <button
                onClick={handleMinimize}
                className="w-3 h-3 rounded-full bg-yellow-500 hover:bg-yellow-600 transition-colors"
                aria-label="Minimize"
              />
            </div>
            <p className="text-xl font-semibold" data-tauri-drag-region>Серверы</p>
          </div>
          <div className="flex flex-col gap-2 p-2">
            {servers.map((server) => (
              <Card
                key={server.server.srvid}
                className={`items-center justify-center cursor-pointer py-2! gap-2 transition-colors ${selectedServer?.server.srvid === server.server.srvid ? "bg-white/10" : ""}`}
                onClick={() => setSelectedServer(server)}
              >
                <img src={server.server.icon} className="w-10 h-10 rounded-lg" alt={server.server.srvName} />
                <div className="flex flex-col items-center justify-center text-center">
                  <span className="text-sm font-bold">{server.server.srvName}</span>
                  <span className="text-xs text-muted-foreground">{server.server.description || "No description"}</span>
                </div>
              </Card>
            ))}
          </div>
        </SidebarContent>
        <SidebarFooter>
          <Button
            onClick={() => setShowAddModal(true)}
            className="w-full"
          >
            Добавить сервер
          </Button>
        </SidebarFooter>
      </Sidebar>

      <Dialog open={showAddModal} onOpenChange={setShowAddModal}>
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle>Добавить сервер</DialogTitle>
            <DialogDescription>
              Введите ID сервера GDPS. Нажмите установить, когда закончите.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4">
            <div className="flex gap-4">
              <Label htmlFor="url">
                ID
              </Label>
              <Input
                id="url"
                type="text"
                autoCorrect="off"
                placeholder="например, 7650"
                value={newServerUrl}
                onChange={(e) => setNewServerUrl(e.target.value)}
                disabled={patching}
              />
            </div>
            {patching && <p className="text-sm text-gray-500 text-center">{patchStatus}</p>}
          </div>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setShowAddModal(false)} disabled={patching}>
              Отмена
            </Button>
            <Button onClick={handleAddServer} disabled={patching}>
              {patching ? "Установка..." : "Установить"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <div className="flex flex-col w-full h-full items-center justify-center p-8 relative overflow-hidden pointer-events-none">
        {selectedServer ? (
          <div className="flex flex-col items-center gap-6 relative z-10 pointer-events-auto">
            <img src={selectedServer.server.icon} className="w-32 h-32 rounded-3xl shadow-2xl" alt={selectedServer.server.srvName} />
            <div className="text-center space-y-2">
              <h1 className="text-4xl font-bold tracking-tight text-white drop-shadow-md">{selectedServer.server.srvName}</h1>
              <p className="text-lg text-white/90 drop-shadow-md max-w-md">{selectedServer.server.description}</p>
            </div>
            <Button size="lg" className="w-48 text-lg h-12 shadow-xl" onClick={handleRunGame}>
              Запустить
            </Button>
          </div>
        ) : (
          <div className="text-center text-muted-foreground relative z-10">
            <p>Выберите сервер из списка слева</p>
          </div>
        )}
      </div>
    </SidebarProvider>
  );
}

export default App;
