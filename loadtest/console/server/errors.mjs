export class HttpError extends Error {
  constructor(status,message){super(message);this.status=status;}
}
