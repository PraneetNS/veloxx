"use client";

import { useState, useEffect } from "react";
import { Search, Filter, Terminal, Clock, Sparkles } from "lucide-react";

export default function LogsPage({ params }: { params: { tenant: string } }) {
  const [logs, setLogs] = useState<any[]>([]);
  const [filter, setFilter] = useState({ q: "", level: "", service: "", semantic: false });
  const [loading, setLoading] = useState(false);

  const fetchLogs = async () => {
    setLoading(true);
    const token = localStorage.getItem("veloxx_token");
    const endpoint = filter.semantic ? "search" : "logs";
    let url = `http://localhost:8080/api/v1/${params.tenant}/${endpoint}?q=${filter.q}`;
    if (!filter.semantic) {
      if (filter.level) url += `&level=${filter.level}`;
      if (filter.service) url += `&service=${filter.service}`;
    }

    try {
      const resp = await fetch(url, {
        headers: { "Authorization": `Bearer ${token}` }
      });
      if (resp.ok) {
        const data = await resp.json();
        // search returns result in a different structure
        setLogs(filter.semantic ? data.result : data.data || []);
      }
    } catch (err) {
      console.error("fetch logs failed", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs();
  }, [params.tenant]);

  return (
    <div className="space-y-6 max-w-7xl mx-auto h-full flex flex-col">
      <div className="flex items-center justify-between animate-in slide-in-from-top duration-300">
        <div className="space-y-1">
          <h2 className="text-2xl font-bold text-white flex items-center gap-3">
            <Terminal size={24} className="text-blue-500" />
            Logs
          </h2>
          <p className="text-xs text-gray-400 font-medium uppercase tracking-widest">Global Telemetry Stream</p>
        </div>

        <div className="flex items-center gap-4 bg-[#0d0f12] p-1.5 rounded-xl border border-[#1e2227]">
          <button 
            onClick={() => setFilter({ ...filter, semantic: false })}
            className={`px-4 py-2 rounded-lg text-xs font-bold transition-all ${!filter.semantic ? 'bg-blue-600 text-white shadow-lg' : 'text-gray-500'}`}
          >
            Pattern Search
          </button>
          <button 
            onClick={() => setFilter({ ...filter, semantic: true })}
            className={`px-4 py-2 rounded-lg text-xs font-bold flex items-center gap-2 transition-all ${filter.semantic ? 'bg-indigo-600 text-white shadow-lg' : 'text-gray-500'}`}
          >
            <Sparkles size={14} className={filter.semantic ? 'text-indigo-200' : 'text-gray-500'} />
            AI Semantic
          </button>
        </div>
      </div>

      {/* Control Bar */}
      <div className="veloxx-card p-6 flex flex-wrap gap-4 items-center">
        <div className="flex-1 min-w-[300px] flex items-center bg-[#020305] border border-[#1e2227] rounded-lg px-4 py-2 group focus-within:border-blue-500/50 transition-all">
          <Search size={16} className="text-gray-500" />
          <input
            type="text"
            placeholder={filter.semantic ? "Search by meaning..." : "Search logs by keyword..."}
            className="bg-transparent border-none outline-none text-sm ml-3 w-full text-white placeholder-gray-500 font-mono"
            value={filter.q}
            onChange={(e) => setFilter({ ...filter, q: e.target.value })}
            onKeyDown={(e) => e.key === "Enter" && fetchLogs()}
          />
        </div>

        <div className="flex items-center gap-4">
          <select 
            className="bg-[#020305] border border-[#1e2227] rounded-lg px-4 py-2 text-xs font-medium text-gray-400 outline-none hover:border-gray-600 focus:border-blue-500 transition-all uppercase tracking-widest"
            value={filter.level}
            onChange={(e) => setFilter({ ...filter, level: e.target.value })}
          >
            <option value="">All Levels</option>
            <option value="ERROR">Error</option>
            <option value="WARN">Warn</option>
            <option value="INFO">Info</option>
            <option value="DEBUG">Debug</option>
          </select>
          <button 
            onClick={fetchLogs}
            disabled={loading}
            className="veloxx-btn-primary px-6 text-xs uppercase tracking-[0.2em] font-black"
          >
            {loading ? "querying..." : "Search"}
          </button>
        </div>
      </div>

      {/* Logs Table */}
      <div className="flex-1 veloxx-card overflow-hidden flex flex-col border border-[#1e2227]/30">
        <div className="bg-[#1e2227]/30 border-b border-[#1e2227] px-6 py-4 flex items-center text-[10px] font-bold text-gray-500 uppercase tracking-[0.2em]">
          <div className="w-1/6">Timestamp</div>
          <div className="w-12">Level</div>
          <div className="w-1/6">Service</div>
          <div className="flex-1">Message</div>
        </div>

        <div className="flex-1 overflow-y-auto font-mono text-xs divide-y divide-[#1e2227]/30">
          {logs.length > 0 ? logs.map((log, i) => {
            const row = filter.semantic ? log.payload : log;
            const levelColor = row.level === "ERROR" ? "text-red-500" : row.level === "WARN" ? "text-yellow-500" : "text-blue-400";
            return (
              <div key={i} className="px-6 py-4 flex items-baseline hover:bg-white/[0.02] transition-colors group cursor-text">
                <div className="w-1/6 text-gray-600 font-medium">
                  {new Date(row.timestamp * 1000).toISOString().replace('T', ' ').substring(0, 19)}
                </div>
                <div className={`w-12 font-black ${levelColor} drop-shadow-[0_0_8px_currentColor]`}>
                  {row.level.substring(0, 4)}
                </div>
                <div className="w-1/6 text-gray-300 font-bold truncate pr-4">
                  {row.service}
                </div>
                <div className="flex-1 text-gray-100 break-all select-all group-hover:text-blue-100 transition-colors">
                  {row.message}
                </div>
              </div>
            );
          }) : (
            <div className="flex flex-col items-center justify-center p-20 opacity-30 grayscale text-center space-y-4">
                <Terminal size={48} className="text-gray-700 mx-auto" strokeWidth={1} />
                <p className="text-xs uppercase tracking-widest font-black text-gray-400">No signals found in the current stream</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
