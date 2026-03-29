"use client";

import Link from "next/navigation";
import { usePathname } from "next/navigation";
import { 
  LayoutDashboard, 
  Terminal, 
  Activity, 
  Bell, 
  Bot, 
  Settings, 
  Search,
  LogOut 
} from "lucide-react";
import React from "react";

const NAV_ITEMS = [
  { label: "Overview", icon: LayoutDashboard, href: "/overview" },
  { label: "Logs", icon: Terminal, href: "/logs" },
  { label: "Metrics", icon: Activity, href: "/metrics" },
  { label: "Alerts", icon: Bell, href: "/alerts" },
  { label: "AI Chat", icon: Bot, href: "/ai-chat" },
];

export default function TenantLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: { tenant: string };
}) {
  const pathname = usePathname();

  return (
    <div className="flex h-screen bg-[#020305] text-[#e1e3e6]">
      {/* Sidebar */}
      <aside className="w-64 flex flex-col border-r border-[#1e2227] bg-[#0d0f12]/50 backdrop-blur-xl shrink-0">
        <div className="p-8">
          <Link href={`/${params.tenant}/overview`} className="flex items-center gap-2 group">
            <div className="bg-blue-600 w-8 h-8 rounded-lg flex items-center justify-center text-white font-bold group-hover:scale-110 transition-all shadow-[0_0_10px_rgba(37,99,235,0.5)]">
              V
            </div>
            <span className="text-xl font-bold tracking-tight text-white group-hover:translate-x-1 transition-transform">
              Veloxx
            </span>
          </Link>
        </div>

        <nav className="flex-1 px-4 space-y-1 overflow-y-auto mt-4">
          <div className="text-xs font-semibold text-gray-500 uppercase px-4 mb-4">
            Observability
          </div>
          {NAV_ITEMS.map((item) => {
            const isActive = pathname.includes(item.href);
            return (
              <a
                key={item.href}
                href={`/${params.tenant}${item.href}`}
                className={`flex items-center gap-3 px-4 py-3 rounded-xl transition-all ${
                  isActive
                    ? "bg-blue-600/10 text-blue-400 border border-blue-600/20"
                    : "text-gray-400 hover:bg-[#1e2227] hover:text-white"
                }`}
              >
                <item.icon size={20} className={isActive ? "text-blue-500" : "text-gray-500"} />
                <span className="font-medium text-sm">{item.label}</span>
              </a>
            );
          })}
        </nav>

        {/* User bar */}
        <div className="p-4 border-t border-[#1e2227] bg-black/20">
          <div className="flex items-center gap-3 p-3 rounded-lg hover:bg-[#1e2227] transition-all cursor-pointer">
            <div className="w-10 h-10 rounded-full bg-gradient-to-tr from-blue-600 to-indigo-800 flex items-center justify-center text-xs font-medium border border-white/10">
              AD
            </div>
            <div className="flex-1 overflow-hidden">
              <p className="text-sm font-semibold truncate leading-none mb-1 text-white">Admin</p>
              <p className="text-[10px] text-gray-500 truncate">{params.tenant}</p>
            </div>
            <LogOut size={16} className="text-gray-600" />
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col min-w-0 overflow-hidden relative">
        {/* Header backdrop blur effect */}
        <header className="h-16 flex items-center justify-between px-10 border-b border-[#1e2227] bg-[#020305]/80 backdrop-blur-md sticky top-0 z-50">
          <div className="flex items-center bg-[#1e2227] rounded-lg border border-[#30363d] px-4 py-2 w-96 group focus-within:border-blue-500/50 transition-all">
            <Search size={18} className="text-gray-500" />
            <input
              type="text"
              placeholder="Natural language search (logs, metrics...)"
              className="bg-transparent border-none outline-none text-sm ml-3 w-full text-white placeholder-gray-500"
            />
          </div>

          <div className="flex items-center gap-6">
            <div className="flex items-center gap-2 px-3 py-1 bg-green-500/10 border border-green-500/20 rounded-full">
              <div className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse" />
              <span className="text-[10px] font-bold text-green-500 uppercase tracking-widest">Live</span>
            </div>
            <button className="text-gray-400 hover:text-white transition-all p-1">
              <Bell size={20} />
            </button>
            <button className="text-gray-400 hover:text-white transition-all p-1">
              <Settings size={20} />
            </button>
          </div>
        </header>

        <section className="flex-1 overflow-y-auto p-10 bg-[radial-gradient(circle_at_top_right,_var(--tw-gradient-stops))] from-blue-900/5 via-transparent to-transparent">
          {children}
        </section>
      </main>
    </div>
  );
}
