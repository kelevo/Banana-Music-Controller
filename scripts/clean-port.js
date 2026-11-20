import { execSync } from 'node:child_process';
import os from 'node:os';

const PORT = 1420;

try {
  if (os.platform() === 'win32') {
    execSync(
      `powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort ${PORT} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess -Unique | ForEach-Object { Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }"`,
      { stdio: 'ignore' }
    );
  } else {
    // macOS / Linux
    try {
      const output = execSync(`lsof -ti :${PORT}`, { encoding: 'utf8' }).trim();
      if (output) {
        const pids = output.split('\n').map((p) => p.trim()).filter(Boolean);
        for (const pid of pids) {
          try {
            process.kill(Number(pid), 'SIGKILL');
          } catch (_) {}
        }
      }
    } catch (_) {
      // Si lsof no encontró procesos, termina con código 1, lo cual es normal
    }
  }
} catch (_) {
  // Ignorar errores si no había nada escuchando en el puerto
}
