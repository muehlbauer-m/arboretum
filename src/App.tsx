import { useState } from "react";
import Sidebar from "./components/Sidebar";
import Home from "./pages/Home";
import Scanner from "./pages/Scanner";
import History from "./pages/History";
import Settings from "./pages/Settings";
import Help from "./pages/Help";
import { GenerationProvider } from "./lib/GenerationContext";
import { ConflictProvider } from "./lib/ConflictContext";
import { LocalPullProvider } from "./lib/LocalPullContext";
import type { Page } from "./lib/types";

export default function App() {
  const [page, setPage] = useState<Page>("home");

  return (
    <LocalPullProvider>
      <GenerationProvider>
        <ConflictProvider>
          <div className="flex h-screen bg-canvas overflow-hidden">
            <Sidebar currentPage={page} onNavigate={setPage} />
            <main className="flex-1 overflow-hidden">
              {page === "home" && <Home />}
              {page === "scanner" && <Scanner />}
              {page === "history" && <History />}
              {page === "settings" && <Settings />}
              {page === "help" && <Help />}
            </main>
          </div>
        </ConflictProvider>
      </GenerationProvider>
    </LocalPullProvider>
  );
}
