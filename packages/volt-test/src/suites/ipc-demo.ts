import { MenuAutomationDriver, TrayAutomationDriver } from '../drivers/index.js';
import { VoltAppLauncher } from '../launcher.js';
import { assertWindowReady, parseWindowStatus } from '../window.js';
import type { VoltTestSuite } from '../types.js';

import { RESULT_FILE, prepareIpcDemoSmokeProject } from './ipc-demo/project.js';
import type { IpcDemoSmokePayload, IpcDemoSmokeSuiteOptions } from './ipc-demo/types.js';
import { validateIpcDemoPayload } from './ipc-demo/payload.js';

export type { IpcDemoSmokePayload, IpcDemoSmokeSuiteOptions } from './ipc-demo/types.js';

export function createIpcDemoSmokeSuite(options: IpcDemoSmokeSuiteOptions = {}): VoltTestSuite {
  const name = options.name ?? 'ipc-demo-smoke';
  const projectDir = options.projectDir ?? 'examples/ipc-demo';
  const timeoutMs = options.timeoutMs ?? 120_000;

  return {
    name,
    timeoutMs,
    async run(context) {
      const launcher = new VoltAppLauncher({
        repoRoot: context.repoRoot,
        cliEntryPath: context.cliEntryPath,
        logger: context.logger,
      });
      const menuDriver = new MenuAutomationDriver();
      const trayDriver = new TrayAutomationDriver();

      const payload = await launcher.run<IpcDemoSmokePayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject: prepareIpcDemoSmokeProject,
        validatePayload: validateIpcDemoPayload,
        artifactsDir: context.artifactsDir,
      });

      const menuSetup = menuDriver.parseSetupPayload(payload.nativeSetup);
      const traySetup = trayDriver.parseSetupPayload(payload.nativeSetup);
      const windowStatus = parseWindowStatus(payload.status);

      if (!menuSetup.shortcutRegistered) {
        context.logger.warn(
          '[volt:test] ipc-demo shortcut registration failed (accepted in headless CI).',
        );
      }

      if (!traySetup.trayReady) {
        context.logger.warn(
          '[volt:test] ipc-demo tray setup reported trayReady=false (accepted in headless CI).',
        );
      }

      const menuClicks = menuDriver.countClickEvents(payload.events);
      const trayClicks = trayDriver.countClickEvents(payload.events);
      assertWindowReady(windowStatus, 1);
      context.logger.log(
        `[volt:test] ipc-demo event summary: menuClicks=${menuClicks}, trayClicks=${trayClicks}, totalEvents=${payload.events.length}`,
      );
      await context.captureScreenshot(`${name}-post-run`);
    },
  };
}
