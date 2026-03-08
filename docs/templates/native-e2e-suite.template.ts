import {
  FileDialogAutomationDriver,
  VoltAppLauncher,
  assertWindowReady,
  parseWindowStatus,
  type VoltTestSuite,
} from '@voltkit/volt-test';

interface NativeTemplatePayload {
  ok: boolean;
  status: unknown;
  openDialogResult: unknown;
}

export function createNativeTemplateSuite(): VoltTestSuite {
  return {
    name: 'native-template-smoke',
    timeoutMs: 120_000,
    async run(context) {
      const launcher = new VoltAppLauncher({
        repoRoot: context.repoRoot,
        cliEntryPath: context.cliEntryPath,
        logger: context.logger,
      });
      const fileDialogDriver = new FileDialogAutomationDriver();

      const payload = await launcher.run<NativeTemplatePayload>({
        sourceProjectDir: 'examples/your-app',
        resultFile: '.volt-smoke-result.json',
        timeoutMs: context.timeoutMs,
        artifactsDir: context.artifactsDir,
        validatePayload: (raw) => raw as NativeTemplatePayload,
      });

      const windowStatus = parseWindowStatus(payload.status);
      assertWindowReady(windowStatus, 1);

      const openResult = fileDialogDriver.parseOpenDialogResult(payload.openDialogResult);
      if (!openResult.canceled) {
        fileDialogDriver.assertOpenSelection(openResult, openResult.filePaths);
      }

      await context.captureScreenshot(`${context.suiteName}-final`);
    },
  };
}
