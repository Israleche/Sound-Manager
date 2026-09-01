using System;
using System.IO;
using System.Linq;
using System.Diagnostics;
using System.Text;
using Newtonsoft.Json;

namespace SoundManager
{
    /// <summary>
    /// SoundManager CLI engine — headless replacement for the WinForms UI.
    /// Exposes the entire sound-scheme engine as JSON over stdout so any UI
    /// (Tauri, web, terminal) can drive it. Two modes:
    ///   - JSON mode (default): reads one JSON command from stdin, prints JSON result, exits.
    ///   - --bg-sounds: launches the hidden background sound player (Windows only).
    ///   - --setup / --uninstall: legacy automated setup/uninstall (kept for installers).
    /// </summary>
    internal static class Program
    {
        private static readonly object _writeLock = new object();

        private static int Main(string[] args)
        {
            // Debug logging
            if (args.Contains(RuntimeConfig.CmdArgumentDebug))
                ExceptionLogger.StartLogging(RuntimeConfig.AppInternalName + ".debug.log", RuntimeConfig.Version);

            try
            {
                // ---- Background sound player (Windows only) ----
                if (args.Contains(RuntimeConfig.CmdArgumentBgSoundPlayer))
                    return RunBgSoundPlayer();

                // ---- Legacy automated setup/uninstall ----
                if (args.Length > 0)
                {
                    switch (args[0])
                    {
                        case RuntimeConfig.CmdArgumentSetup:
                            SoundManager.Core.Program.Setup(forceResetSounds: false, systemIntegration: true, offerImportCurrentScheme: false);
                            return 0;
                        case RuntimeConfig.CmdArgumentUninstall:
                            SoundManager.Core.Program.Uninstall();
                            return 0;
                    }
                }

                // ---- JSON command mode ----
                // Reads a single JSON envelope from stdin, dispatches, writes JSON result.
                string input;
                using (var sr = new StreamReader(Console.OpenStandardInput(), Encoding.UTF8))
                    input = sr.ReadToEnd();

                var request = string.IsNullOrWhiteSpace(input)
                    ? new JsonRpcRequest { Command = "ping" }
                    : JsonConvert.DeserializeObject<JsonRpcRequest>(input.Trim());

                var result = CommandRouter.Dispatch(request);
                WriteJson(result);
                return result.Error == null ? 0 : 1;
            }
            catch (Exception ex)
            {
                WriteJson(new JsonRpcResponse
                {
                    Id = null,
                    Error = new JsonRpcError { Code = -1, Message = ex.Message, Data = ex.StackTrace }
                });
                return 1;
            }
        }

        private static int RunBgSoundPlayer()
        {
            // BgSoundPlayer needs Windows message pump — launch a hidden WinForms window.
            // On non-Windows this is a no-op.
            if (!WindowsVersion.IsWindows)
            {
                WriteJson(new JsonRpcResponse
                {
                    Id = null,
                    Error = new JsonRpcError { Code = -32602, Message = "Background sound player requires Windows." }
                });
                return 1;
            }

            var bg = new BgSoundPlayer();
            SystemEvents.SessionEnding += (s, e) => bg.OnSessionEnding(e);
            SystemEvents.SessionSwitch += (s, e) => bg.OnSessionSwitch(e);
            Application.Run(bg);
            return 0;
        }

        private static void WriteJson(JsonRpcResponse response)
        {
            var json = JsonConvert.SerializeObject(response, Formatting.None,
                new JsonSerializerSettings { NullNullHandling = NullHandling.Ignore });
            lock (_writeLock)
            {
                Console.WriteLine(json);
            }
        }
    }
}