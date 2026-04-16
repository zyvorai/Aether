import { useState, type FormEvent } from 'react';
import { Lock, User, Hexagon } from 'lucide-react';

interface LoginProps {
  onLogin: (username: string) => void;
}

export default function Login({ onLogin }: LoginProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');

  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (username === 'admin' && password === 'aether') {
      onLogin(username);
    } else {
      setError('Invalid credentials. Try admin / aether');
    }
  }

  const runtimes = [
    { name: 'Podman', color: 'bg-blue-500/10 text-blue-400 border-blue-500/20' },
    { name: 'Kubernetes', color: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' },
    { name: 'KubeVirt', color: 'bg-purple-500/10 text-purple-400 border-purple-500/20' },
    { name: 'Metal3', color: 'bg-red-500/10 text-red-400 border-red-500/20' },
  ];

  return (
    <div className="min-h-screen bg-zinc-950 flex items-center justify-center p-4">
      <div className="w-full max-w-md animate-fade-in">
        <div className="bg-zinc-900 border border-zinc-700 rounded-2xl p-8 shadow-2xl card-metal">
          {/* Logo */}
          <div className="flex flex-col items-center mb-8">
            <div className="w-16 h-16 rounded-xl bg-aether/10 flex items-center justify-center mb-4">
              <Hexagon className="w-9 h-9 text-aether" />
            </div>
            <h1 className="text-2xl font-bold text-white">Aether</h1>
            <p className="text-zinc-400 text-sm mt-1">Universal Runtime Control Plane</p>
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="username" className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1.5">
                Username
              </label>
              <div className="relative">
                <User className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                <input
                  id="username"
                  type="text"
                  value={username}
                  onChange={(e) => { setUsername(e.target.value); setError(''); }}
                  className="w-full bg-zinc-950 border border-zinc-700 rounded-lg pl-10 pr-4 py-2.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors"
                  placeholder="Enter username"
                  autoComplete="username"
                  autoFocus
                />
              </div>
            </div>

            <div>
              <label htmlFor="password" className="block text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1.5">
                Password
              </label>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" />
                <input
                  id="password"
                  type="password"
                  value={password}
                  onChange={(e) => { setPassword(e.target.value); setError(''); }}
                  className="w-full bg-zinc-950 border border-zinc-700 rounded-lg pl-10 pr-4 py-2.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors"
                  placeholder="Enter password"
                  autoComplete="current-password"
                />
              </div>
            </div>

            {error && (
              <div className="bg-red-500/10 border border-red-500/20 rounded-lg px-4 py-2.5 text-sm text-red-400 animate-fade-in">
                {error}
              </div>
            )}

            <button
              type="submit"
              className="w-full bg-aether hover:bg-aether-light text-white font-medium rounded-lg py-2.5 text-sm transition-colors focus:outline-none focus:ring-2 focus:ring-aether/50"
            >
              Sign In
            </button>
          </form>

          {/* Runtime pills */}
          <div className="mt-8 pt-6 border-t border-zinc-700">
            <p className="text-xs text-zinc-500 text-center mb-3">Supported Runtimes</p>
            <div className="flex flex-wrap justify-center gap-2">
              {runtimes.map((rt) => (
                <span
                  key={rt.name}
                  className={`text-xs px-3 py-1 rounded-full border ${rt.color}`}
                >
                  {rt.name}
                </span>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
