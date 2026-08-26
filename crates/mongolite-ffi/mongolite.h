#ifndef MONGOLITE_H
#define MONGOLITE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Database Database;

Database* mongolite_create(const char* path);
Database* mongolite_open(const char* path);
int mongolite_close(Database* db);
int mongolite_insert(Database* db, const char* collection, const char* json, char* out_id, int out_id_len);
int mongolite_find(Database* db, const char* collection, const char* filter, char* out_json, int out_json_len);
int mongolite_count(Database* db, const char* collection);
int mongolite_delete(Database* db, const char* collection, const char* filter);

#ifdef __cplusplus
}
#endif

#endif
