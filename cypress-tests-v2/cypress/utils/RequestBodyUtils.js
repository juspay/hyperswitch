export function isoTimeTomorrow() {
  const now = new Date();

  // Create a new date object for tomorrow
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);

  // Convert to ISO string format
  const isoStringTomorrow = tomorrow.toISOString();
  return isoStringTomorrow;
}
