
    CREATE TABLE "tasks" (
        "id"	INTEGER NOT NULL,
        "title"	TEXT NOT NULL,
        "description"	TEXT NOT NULL,
        "create_date"	TEXT NOT NULL,
        "status"	TEXT NOT NULL,
        "deleted"	INTEGER NOT NULL,
        PRIMARY KEY("id" AUTOINCREMENT)
    );