// SQLite CURRENT_TIMESTAMP values are UTC without a zone designator; append 'Z' so
// they are parsed as UTC instead of local time.
export const parseUTCTimestamp = (dateString: string): Date => {
  const utcDateString = dateString.endsWith('Z') ? dateString : `${dateString}Z`;
  return new Date(utcDateString);
};
