defmodule Fortunes.Repo do
  use Ecto.Repo,
    otp_app: :fortunes,
    adapter: Ecto.Adapters.Postgres
end
