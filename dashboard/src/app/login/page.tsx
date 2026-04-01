"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const router = useRouter();

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    try {
      const resp = await fetch("http://localhost:8080/api/v1/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      });

      if (resp.ok) {
        const data = await resp.json();
        localStorage.setItem("veloxx_token", data.access_token);
        localStorage.setItem("veloxx_tenant_id", data.tenant_id);
        router.push(`/${data.tenant_id}/overview`);
      } else {
        const err = await resp.json();
        setError(err.error || "Login failed");
      }
    } catch {
      setError("Server connection failed");
    }
  };

  return (
    <div className="flex h-screen items-center justify-center bg-black">
      <div className="w-full max-w-md glass p-10 space-y-8 animate-in fade-in zoom-in duration-500">
        <div className="text-center space-y-2">
          <h1 className="text-4xl font-bold tracking-tight text-white flex items-center justify-center gap-2">
            <span className="text-blue-500">V</span>eloxx
          </h1>
          <p className="text-gray-400 text-sm italic">AI-Native Observability</p>
        </div>

        <form onSubmit={handleLogin} className="space-y-6">
          <div className="space-y-2">
            <label className="text-sm font-medium text-gray-400">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full bg-[#1e2227] border border-[#30363d] rounded-lg px-4 py-3 text-white outline-none focus:border-blue-500 transition-all"
              required
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium text-gray-400">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full bg-[#1e2227] border border-[#30363d] rounded-lg px-4 py-3 text-white outline-none focus:border-blue-500 transition-all"
              required
            />
          </div>

          {error && <p className="text-red-500 text-sm font-medium">{error}</p>}

          <button type="submit" className="veloxx-btn-primary w-full py-4 text-lg">
            Sign In
          </button>
        </form>

        <p className="text-center text-xs text-gray-600">
          Powered by Veloxx Cloud AI
        </p>
      </div>
    </div>
  );
}
