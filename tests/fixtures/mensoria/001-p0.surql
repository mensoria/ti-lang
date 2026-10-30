-- P0 fixtures and contracts only; not a complete production schema.
DEFINE TABLE user SCHEMAFULL PERMISSIONS FOR select WHERE id = $auth.id;
DEFINE FIELD email ON user TYPE string;
DEFINE FIELD pass ON user TYPE string PERMISSIONS NONE;
DEFINE FIELD active ON user TYPE bool DEFAULT true;
DEFINE FIELD role ON user TYPE string DEFAULT 'member';
DEFINE INDEX user_email ON user FIELDS email UNIQUE;
DEFINE ACCESS account ON DATABASE TYPE RECORD
    SIGNIN (SELECT * FROM user WHERE email = $email AND active = true AND crypto::argon2::compare(pass, $pass))
    DURATION FOR TOKEN 5m, FOR SESSION 5m;

DEFINE TABLE doc SCHEMAFULL
    PERMISSIONS FOR select, create, update, delete WHERE owner = $auth.id AND $auth.active = true;
DEFINE FIELD title ON doc TYPE string;
DEFINE FIELD owner ON doc TYPE record<user>;
DEFINE FIELD secret ON doc TYPE option<string> PERMISSIONS FOR select WHERE $auth.role = 'admin';
DEFINE TABLE operation SCHEMALESS PERMISSIONS NONE;
DEFINE TABLE outbox SCHEMALESS PERMISSIONS NONE;
DEFINE TABLE job SCHEMALESS PERMISSIONS NONE;
DEFINE TABLE environment_guard SCHEMALESS PERMISSIONS NONE;
DEFINE TABLE safety_tombstone SCHEMALESS PERMISSIONS NONE;
CREATE environment_guard:current SET incarnation = 1;

-- Record users can only touch the contention counter; they cannot open/reseed the guard.
DEFINE TABLE OVERWRITE environment_guard SCHEMAFULL
    PERMISSIONS FOR select WHERE $auth.id != NONE
    FOR update WHERE $auth.active = true AND state = 'OPEN';
DEFINE FIELD incarnation ON environment_guard TYPE int PERMISSIONS FOR select FULL FOR create, update NONE;
DEFINE FIELD applied_security_seq ON environment_guard TYPE int PERMISSIONS FOR select FULL FOR create, update NONE;
DEFINE FIELD state ON environment_guard TYPE string PERMISSIONS FOR select FULL FOR create, update NONE;
DEFINE FIELD revision ON environment_guard TYPE int;
DEFINE FIELD management_generation ON environment_guard TYPE int DEFAULT 0 PERMISSIONS FOR select FULL FOR create, update NONE;
UPDATE environment_guard:current SET state = 'OPEN', revision = 0, applied_security_seq = 0, management_generation=0;
DEFINE TABLE security_receipt SCHEMALESS PERMISSIONS NONE;
