import { doctorCommand, __testOnly } from './doctor/command.js';

export { doctorCommand };
export type {
  DoctorCheckResult,
  DoctorCheckStatus,
  DoctorOptions,
  DoctorPlatform,
  DoctorReport,
} from './doctor/command.js';

export const doctorTestOnly = __testOnly;
