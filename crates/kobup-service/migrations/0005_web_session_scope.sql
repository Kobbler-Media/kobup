-- Web sessions now belong to users of any role, so their scope no longer says "admin".
UPDATE auth_tokens SET scope = 'session' WHERE scope = 'admin';
