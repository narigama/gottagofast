defmodule FortunesWeb.Router do
  use FortunesWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", FortunesWeb do
    pipe_through :api

    post "/fortunes", FortuneController, :show
  end
end
