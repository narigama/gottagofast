import contextlib
import multiprocessing
import asyncpg
import fastapi
import pydantic
import pydantic_settings


MAX_CORES = multiprocessing.cpu_count()


class Config(pydantic_settings.BaseSettings):
    # load .env
    model_config = pydantic_settings.SettingsConfigDict(env_file=".env", env_file_encoding="utf-8")

    # asyncpg pool
    database_url: pydantic.PostgresDsn
    database_pool_min_size: int = (MAX_CORES // 2) or 1
    database_pool_max_size: int = (MAX_CORES) or 1


async def get_database(request: fastapi.Request) -> asyncpg.Connection:
    """
    Mock this function in tests to avoid committing changes.
    """
    async with request.app.state.database_pool.acquire() as conn, conn.transaction():
        yield conn


class Model(pydantic.BaseModel):
    pass


class Fortune(Model):
    content: str


class FortuneListRequest(Model):
    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "quantity": 5,
                }
            ]
        }
    }

    quantity: int = pydantic.Field(5, gt=0, lt=21)


class FortuneListResponse(Model):
    count: int
    items: list[Fortune]


async def get_fortunes(db: asyncpg.Connection, n: int) -> list[Fortune]:
    query = """
        SELECT "content"
        FROM "fortune"
        ORDER BY random()
        LIMIT $1
    """
    return [Fortune(content=row["content"]) for row in await db.fetch(query, n)]


async def fortunes(
    request: FortuneListRequest,
    db: asyncpg.Connection = fastapi.Depends(get_database),
):
    """
    Return 5 fortunes at random. If quantity=N is passed via the query string,
    return that many items instead.
    """
    items = await get_fortunes(db, request.quantity)

    return FortuneListResponse(
        count=len(items),
        items=items,
    )


@contextlib.asynccontextmanager
async def lifespan(app: fastapi.FastAPI):
    app.state.database_pool = await asyncpg.create_pool(
        dsn=app.state.config.database_url.unicode_string(),
        min_size=app.state.config.database_pool_min_size,
        max_size=app.state.config.database_pool_max_size,
    )
    yield
    await app.state.database_pool.close()


def factory():
    app = fastapi.FastAPI(lifespan=lifespan)
    app.state.config = Config()
    app.add_api_route(path="/fortunes", endpoint=fortunes, methods=["POST"])

    return app
