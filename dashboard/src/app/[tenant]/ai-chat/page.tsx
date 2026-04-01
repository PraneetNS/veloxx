"use client";

import { useState } from "react";
import { Bot, User, Send, Sparkles, MessageSquare, Database, Terminal, Clock, Activity } from "lucide-react";

type ChatMessage = {
  role: "bot" | "user";
  content: string;
};

export default function AiChatPage({ params }: { params: { tenant: string } }) {
  const [messages, setMessages] = useState<ChatMessage[]>([
    { role: "bot", content: `Hello! I'm Veloxx AI. How can I help you explore your telemetry for tenant ${params.tenant} today?` }
  ]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSend = async () => {
    if (!input.trim() || loading) return;

    setMessages([...messages, { role: "user", content: input }]);
    setLoading(true);
    setInput("");

    const token = localStorage.getItem("veloxx_token");
    try {
      const resp = await fetch(`http://localhost:8080/api/v1/${params.tenant}/ask`, {
        method: "POST",
        headers: { 
          "Authorization": `Bearer ${token}`,
          "Content-Type": "application/json"
        },
        body: JSON.stringify({ question: input }),
      });

      if (resp.ok) {
        const data = await resp.json();
        setMessages((prev) => [...prev, { role: "bot", content: data.answer }]);
      } else {
        setMessages((prev) => [...prev, { role: "bot", content: "Error communicating with AI service" }]);
      }
    } catch {
      setMessages((prev) => [...prev, { role: "bot", content: "Connection to AI service failed" }]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full max-w-5xl mx-auto space-y-10">
        <div className="flex flex-col gap-4 animate-in slide-in-from-top duration-500">
            <h2 className="text-3xl font-bold font-sans text-white tracking-tighter flex items-center gap-4">
                <Sparkles className="text-blue-500 animate-pulse" size={28} />
                AI Intelligence Layer
            </h2>
            <div className="flex items-center gap-6 text-[10px] font-bold text-gray-500 uppercase tracking-widest bg-[#0d0f12] p-2 pr-6 rounded-full border border-white/5 shadow-2xl">
                <div className="flex items-center gap-2 border-r border-[#1e2227] px-4"><Database size={12} className="text-blue-500" /> Clickhouse Core</div>
                <div className="flex items-center gap-2 border-r border-[#1e2227] px-4"><Activity size={12} className="text-green-500" /> Real-time Vectorized</div>
                <div className="flex items-center gap-2 px-4 shadow-[inset_0_0_10px_rgba(37,99,235,0.1)]"><Terminal size={12} className="text-purple-500" /> Explain Context Layer</div>
            </div>
        </div>

      <div className="flex-1 veloxx-card overflow-hidden flex flex-col min-h-[500px] border border-[#1e2227]/30 shadow-[0_0_50px_rgba(0,0,0,0.5)]">
        <div className="flex-1 overflow-y-auto p-10 space-y-8 scroll-smooth">
          {messages.map((m, i) => (
            <div key={i} className={`flex gap-6 ${m.role === "user" ? "flex-row-reverse" : "flex-row"} animate-in fade-in duration-300`}>
              <div className={`w-12 h-12 rounded-xl flex items-center justify-center shrink-0 border border-white/5 ${m.role === "bot" ? "bg-blue-600 shadow-[0_0_15px_rgba(37,99,235,0.4)] text-white" : "bg-[#1e2227] text-gray-400 group-hover:text-white"}`}>
                {m.role === "bot" ? <Bot size={24} /> : <User size={24} />}
              </div>
              <div className={`max-w-[80%] rounded-2xl p-6 shadow-xl leading-relaxed text-sm ${m.role === "bot" ? "bg-[#161a20] text-gray-100 border border-[#1e2227] font-medium" : "bg-blue-600/10 text-blue-100 border border-blue-600/20 font-semibold"}`}>
                {m.content}
                {m.role === "bot" && (
                    <div className="mt-4 pt-4 border-t border-white/5 flex items-center gap-4 text-[10px] font-bold text-gray-600 uppercase tracking-widest">
                        <Clock size={12} strokeWidth={3} /> Resolved from logs + metrics
                    </div>
                )}
              </div>
            </div>
          ))}
          {loading && (
            <div className="flex gap-4 animate-pulse">
                <div className="w-12 h-12 rounded-xl bg-[#161a20] border border-white/5" />
                <div className="max-w-[40%] rounded-2xl bg-[#161a20] p-6 border border-[#1e2227] h-12" />
            </div>
          )}
        </div>

        <div className="p-8 border-t border-[#1e2227] bg-[#020305]/50 backdrop-blur-xl">
          <div className="flex items-center gap-4 bg-[#0d0f12] border border-[#1e2227] rounded-2xl px-6 py-4 focus-within:border-blue-500/50 transition-all shadow-inner group">
            <MessageSquare size={18} className="text-gray-600 group-focus-within:text-blue-500 transition-colors" />
            <input
              type="text"
              placeholder="Ask anything about your system state (e.g. 'Why did error rate spike in service X?')"
              className="bg-transparent border-none outline-none text-sm w-full text-white placeholder-gray-600 font-medium"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSend()}
            />
            <button 
                onClick={handleSend}
                disabled={loading}
                className="p-3 bg-blue-600 hover:bg-blue-500 text-white rounded-xl shadow-lg transition-all active:scale-95 disabled:opacity-50"
            >
              <Send size={18} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
