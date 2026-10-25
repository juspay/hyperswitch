const activeStatuses=['starting','running','pausing','paused','cancelling'];
export function recordingFinished(job) {
 return job?.command==='load'&&job.progress?.control?.state==='completed';
}
export function runJobPresentation(jobStatus,recording) {
 const cleanupPending=activeStatuses.includes(jobStatus)&&recording?.recording_state==='complete'&&Boolean(recording?.ended_at);
 return {jobStatus:cleanupPending?'complete':jobStatus,cleanupPending};
}
