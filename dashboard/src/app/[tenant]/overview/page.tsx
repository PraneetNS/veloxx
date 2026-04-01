"use client";

import { useEffect, useState } from "react";
import { 
  ArrowUpRight, 
  Activity, 
  AlertTriangle, 
  Bot,
  Cpu, 
  Zap, 
  Clock 
} from "lucide-react";

type LiveMetricsPayload = {
  metrics?: unknown[];
};

type Anomaly = {
  id: string;
  severity: string;
  opened_at: string;
  title: string;
  description: string | null;
};

export default function OverviewPage({ params }: { params: { tenant: string } }) {
  const [, setMetrics] = useState<unknown[]>([]);
  const [anomalies, setAnomalies] = useState<Anomaly[]>([]);

  useEffect(() => {
    // 1. WebSocket for live metric updates.
    const ws = new WebSocket(`ws://localhost:8080/api/v1/${params.tenant}/live`);
    ws.onmessage = (event) => {
      const data: LiveMetricsPayload = JSON.parse(event.data);
      if (data.metrics) setMetrics(data.metrics);
    };

    // 2. Fetch recent anomalies.
    const fetchAnomalies = async () => {
      const token = localStorage.getItem("veloxx_token");
      try {
        const resp = await fetch(`http://localhost:8080/api/v1/${params.tenant}/anomalies`, {
          headers: { "Authorization": `Bearer ${token}` }
        });
        if (resp.ok) {
          const data = await resp.json();
          setAnomalies(data.data || []);
        }
      } catch (err) {
        console.error("fetch anomalies failed", err);
      }
    };

    fetchAnomalies();
    const interval = setInterval(fetchAnomalies, 5000);

    return () => {
      ws.close();
      clearInterval(interval);
    };
  }, [params.tenant]);

  return (
    <div className="space-y-10 max-w-7xl mx-auto">
      <div className="flex flex-col gap-4 animate-in slide-in-from-top duration-500">
        <h2 className="text-3xl font-bold text-white tracking-tight flex items-center gap-4">
          Overview
          <div className="bg-[#1e2227] h-px flex-1 mt-1 mx-4" />
        </h2>
        <div className="flex items-center gap-2 text-sm text-gray-500">
          <Activity size={16} className="text-blue-500" />
          <span>Real-time monitoring enabled for </span>
          <span className="text-blue-400 font-mono font-bold uppercase">{params.tenant}</span>
        </div>
      </div>

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        {[
          { label: "Error Rate", value: "0.2%", icon: AlertTriangle, color: "text-red-500", bg: "bg-red-500/10" },
          { label: "P99 Latency", value: "145ms", icon: Clock, color: "text-blue-400", bg: "bg-blue-400/10" },
          { label: "Throughput", value: "1.2k req/s", icon: Zap, color: "text-yellow-500", bg: "bg-yellow-500/10" },
          { label: "CPU Usage", value: "23%", icon: Cpu, color: "text-green-500", bg: "bg-green-500/10" },
        ].map((v) => (
          <div key={v.label} className="veloxx-card p-6 flex items-start justify-between relative overflow-hidden group">
            <div className="z-10 relative">
              <p className="text-sm font-medium text-gray-500 mb-1">{v.label}</p>
              <h3 className="text-2xl font-bold text-white flex items-baseline gap-2">
                {v.value}
                <span className="text-[10px] text-green-500 font-medium">+2.1%</span>
              </h3>
            </div>
            <div className={`p-2 rounded-lg ${v.bg} border border-white/5 group-hover:scale-110 transition-transform`}>
              <v.icon size={20} className={v.color} />
            </div>
            {/* Background sparkle effect */}
            <div className="absolute top-0 right-0 w-32 h-32 bg-blue-500/10 blur-[60px] translate-x-12 -translate-y-12" />
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-10">
        {/* Main Graph Placeholder */}
        <div className="lg:col-span-2 space-y-6">
          <div className="veloxx-card p-8 min-h-[460px] flex flex-col">
            <div className="flex items-center justify-between mb-8">
              <h3 className="text-lg font-bold text-white tracking-wide uppercase">Request Traffic</h3>
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2 text-xs font-medium text-gray-500">
                  <span className="w-2 h-2 rounded-full bg-blue-500" />
                  API
                </div>
                <div className="flex items-center gap-2 text-xs font-medium text-gray-500">
                  <span className="w-2 h-2 rounded-full bg-indigo-500" />
                  Auth
                </div>
              </div>
            </div>
            
            <div className="flex-1 border-b border-l border-[#1e2227] relative bg-[repeating-linear-gradient(90deg,_#1e2227_0,_#1e2227_1px,_transparent_1px,_transparent_40px)]">
              {/* Simple visual indicator of life */}
              <div className="absolute inset-0 flex items-center justify-center">
                <p className="text-xs text-gray-700 italic uppercase tracking-[0.2em]">Real-time Telemetry Graph</p>
              </div>
            </div>
          </div>
        </div>

        {/* Anomaly Feed */}
        <div className="space-y-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-bold text-white uppercase tracking-wider">AI Anomaly Feed</h3>
            <span className="text-[10px] bg-blue-600 px-2 py-0.5 rounded-full text-white font-bold">LIVE</span>
          </div>

          <div className="space-y-4">
            {anomalies.length > 0 ? (
              anomalies.map((a) => (
                <div key={a.id} className="veloxx-card p-4 border-l-2 border-l-blue-500 hover:bg-blue-500/5 transition-colors group cursor-pointer relative">
                  <div className="flex justify-between items-start mb-2">
                    <span className="text-[10px] font-bold text-blue-500 uppercase tracking-widest">{a.severity}</span>
                    <span className="text-[10px] text-gray-600 font-mono italic">
                      {new Date(a.opened_at).toLocaleTimeString()}
                    </span>
                  </div>
                  <h4 className="text-sm font-bold text-white mb-1">{a.title}</h4>
                  <p className="text-xs text-gray-500 line-clamp-2">{a.description}</p>
                  <ArrowUpRight size={14} className="absolute bottom-4 right-4 text-gray-700 group-hover:text-blue-500 transition-colors" />
                </div>
              ))
            ) : (
                <div className="veloxx-card p-10 flex flex-col items-center justify-center text-center opacity-50 grayscale">
                    <Bot size={40} className="mb-4 text-blue-500/20" />
                    <p className="text-xs text-gray-500 uppercase font-bold tracking-widest">Scanning for anomalies...</p>
                </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
