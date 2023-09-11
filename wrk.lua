wrk.method = 'POST'
wrk.path = '/fortunes'
wrk.body = '{"quantity": 20}'
wrk.headers["Content-Type"] = 'application/json'
