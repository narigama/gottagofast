# GottaGoFast

Run `docker compose up -d` to setup a database and apply migrations. Take note
of the random port number assigned to postgres.

Then set `DATABASE_URL=postgres://fortunes:fortunes@localhost:<PORT>/fortunes?sslmode=disable`

Finally start any of the projects, and hit it with
`oha -z10s -m POST -H "Content-Type: application/json" -d '{}' http://localhost:8000/fortunes`