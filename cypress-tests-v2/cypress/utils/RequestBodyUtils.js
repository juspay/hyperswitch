export function generateOrganizationName() {
  const uuid_placeholder = "xxxxxxxx-xxxx";
  const uuid = uuid_placeholder.replace(/[xy]/g, function (characters) {
    const random = (Math.random() * 16) | 0;
    const value = characters === "x" ? random : (random & 0x3) | 0x8;
    return value.toString(16);
  });

  return uuid;
}

export function isoTimeTomorrow() {
  const now = new Date();

  // Create a new date object for tomorrow
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);

  // Convert to ISO string format
  const isoStringTomorrow = tomorrow.toISOString();
  return isoStringTomorrow;
}
